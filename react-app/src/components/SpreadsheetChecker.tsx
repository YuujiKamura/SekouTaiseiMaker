/**
 * Spreadsheet AI Checker - スプレッドシート/ExcelのAIチェック
 *
 * ## 変更履歴
 * - 2026-01-03: 動的フィールド生成（書類タイプをAIが判定、フィールドも動的）
 * - 2026-01-02: Excelファイル対応（SheetJSでパース）
 * - 2026-01-02: シート選択UI追加
 * - 2026-01-02: 初期実装
 */

// ビルドバージョン（デバッグ用）
const BUILD_VERSION = '2026-01-03T10:10:00';
import { useState, useEffect } from 'react';
import { getApiKey } from '../services/apiKey';
import { safeBase64ToArrayBuffer } from '../utils/base64';
import {
  extractFields,
  generateCellAddressedDataFromArray,
  KEY_FIELDS,
  type ExtractionResult,
  type ProjectContext,
} from '../services/fieldExtractor';
import * as XLSX from 'xlsx';
import './AiChecker.css';

interface SheetInfo {
  sheetId: number;
  name: string;
  rowCount: number;
  colCount: number;
  preview: string[][];
  worksheet?: XLSX.WorkSheet;   // Excel用: SheetJSワークシート
  fullData?: string[][];        // Spreadsheet用: 全データ
}

interface SheetListResponse {
  success?: boolean;
  error?: string;
  spreadsheetId: string;
  spreadsheetName: string;
  sheets: SheetInfo[];
}

interface ExcelResponse {
  success?: boolean;
  error?: string;
  fileId: string;
  fileName: string;
  base64: string;
}

function getUrlParam(name: string): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get(name);
}

export function SpreadsheetChecker() {
  // フェーズ: 'select' | 'check'
  const [phase, setPhase] = useState<'select' | 'check'>('select');
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<ExtractionResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  // シート選択フェーズ用
  const [sheetList, setSheetList] = useState<SheetInfo[]>([]);
  const [spreadsheetName, setSpreadsheetName] = useState('');
  const [selectedSheetIds, setSelectedSheetIds] = useState<Set<number>>(new Set());

  // チェックフェーズ用
  const [sheetData, setSheetData] = useState<string[][] | null>(null);
  const [currentSheetName, setCurrentSheetName] = useState('');

  // Excel用のワークブック
  const [excelWorkbook, setExcelWorkbook] = useState<XLSX.WorkBook | null>(null);

  // 工事情報（妥当性判定用）
  const [projectInfo, setProjectInfo] = useState<{
    projectName?: string;
    periodStart?: string;
    periodEnd?: string;
    siteRepresentative?: string;
    today?: string;
  } | null>(null);

  const spreadsheetId = getUrlParam('spreadsheetId');
  const fileId = getUrlParam('fileId');
  const isExcel = getUrlParam('isExcel') === 'true';
  const gid = getUrlParam('gid');
  const docType = getUrlParam('docType') || '書類';
  const contractor = getUrlParam('contractor') || '業者';
  const gasUrl = getUrlParam('gasUrl');
  const contractorId = getUrlParam('contractorId') || '';
  const docKey = getUrlParam('docKey') || docType;

  // データ取得（Excel or スプレッドシート）
  useEffect(() => {
    if (!gasUrl) {
      setError('GAS URLが設定されていません');
      setLoading(false);
      return;
    }

    if (isExcel && fileId) {
      fetchExcelFile();
    } else if (spreadsheetId) {
      fetchSheetList();
    } else {
      setError('ファイルIDが指定されていません');
      setLoading(false);
    }
  }, [spreadsheetId, fileId, isExcel, gasUrl]);

  // 工事情報取得（妥当性判定用）
  useEffect(() => {
    if (!gasUrl) return;

    const fetchProjectInfo = async () => {
      try {
        const url = `${gasUrl}?action=getProjectInfo`;
        const response = await fetch(url, { cache: 'no-store' });
        const data = await response.json();

        if (data.success) {
          setProjectInfo({
            projectName: data.projectName,
            periodStart: data.periodStart,
            periodEnd: data.periodEnd,
            siteRepresentative: data.siteRepresentative,
            today: data.today,
          });
          console.log('[SpreadsheetChecker] Project info:', data);
        }
      } catch (e) {
        console.warn('[SpreadsheetChecker] Failed to fetch project info:', e);
      }
    };

    fetchProjectInfo();
  }, [gasUrl]);

  const fetchExcelFile = async () => {
    try {
      const url = `${gasUrl}?action=fetchExcelAsBase64&fileId=${encodeURIComponent(fileId!)}`;
      console.log('[SpreadsheetChecker] Fetching Excel:', url);
      const response = await fetch(url, { cache: 'no-store' });
      const data: ExcelResponse = await response.json();

      if (data.error) {
        throw new Error(data.error);
      }

      if (!data.base64) {
        throw new Error('Excelデータを取得できませんでした');
      }

      const bytes = new Uint8Array(safeBase64ToArrayBuffer(data.base64));
      const workbook = XLSX.read(bytes, { type: 'array' });
      setExcelWorkbook(workbook);
      setSpreadsheetName(data.fileName);

      // シート一覧を構築（フィールド抽出なし、シート名とプレビューのみ）
      const sheets: SheetInfo[] = workbook.SheetNames.map((name, index) => {
        const sheet = workbook.Sheets[name];
        const jsonData = XLSX.utils.sheet_to_json<string[]>(sheet, { header: 1 }) as string[][];
        const rowCount = jsonData.length;
        const colCount = jsonData[0]?.length || 0;

        // プレビュー（先頭3行5列）
        const preview = jsonData.slice(0, 3).map(row =>
          (row || []).slice(0, 5).map(cell => String(cell ?? ''))
        );

        return {
          sheetId: index,
          name,
          rowCount,
          colCount,
          preview,
          worksheet: sheet,
        };
      });

      setSheetList(sheets);
      setLoading(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Excelファイル取得エラー');
      setLoading(false);
    }
  };

  const fetchSheetList = async () => {
    try {
      const url = `${gasUrl}?action=listSheets&spreadsheetId=${encodeURIComponent(spreadsheetId!)}`;
      const response = await fetch(url, { cache: 'no-store' });
      const data: SheetListResponse = await response.json();

      if (data.error) {
        throw new Error(data.error);
      }

      setSheetList(data.sheets);
      setSpreadsheetName(data.spreadsheetName);

      if (gid) {
        const gidNum = parseInt(gid, 10);
        if (data.sheets.some(s => s.sheetId === gidNum)) {
          setSelectedSheetIds(new Set([gidNum]));
        }
      }

      setLoading(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'シート一覧取得エラー');
      setLoading(false);
    }
  };

  const toggleSheetSelection = (sheetId: number) => {
    setSelectedSheetIds(prev => {
      const next = new Set(prev);
      if (next.has(sheetId)) {
        next.delete(sheetId);
      } else {
        next.add(sheetId);
      }
      return next;
    });
  };

  const handleProceedToCheck = async () => {
    if (selectedSheetIds.size === 0) {
      setError('チェックするシートを選択してください');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const allData: string[][] = [];
      const sheetNames: string[] = [];

      if (isExcel && excelWorkbook) {
        for (const sheetId of selectedSheetIds) {
          const sheetName = excelWorkbook.SheetNames[sheetId];
          const sheet = excelWorkbook.Sheets[sheetName];
          const jsonData = XLSX.utils.sheet_to_json<string[]>(sheet, { header: 1 });

          if (allData.length > 0) {
            allData.push(['']);
            allData.push([`=== ${sheetName} ===`]);
          } else {
            allData.push([`=== ${sheetName} ===`]);
          }

          jsonData.forEach(row => {
            allData.push((row as string[]).map(cell => String(cell ?? '')));
          });
          sheetNames.push(sheetName);
        }
      } else {
        for (const sheetId of selectedSheetIds) {
          const url = `${gasUrl}?action=fetchSpreadsheet&spreadsheetId=${encodeURIComponent(spreadsheetId!)}&gid=${sheetId}`;
          const response = await fetch(url, { cache: 'no-store' });
          const data = await response.json();

          if (data.error) {
            throw new Error(data.error);
          }

          if (allData.length > 0) {
            allData.push(['']);
            allData.push([`=== ${data.sheetName} ===`]);
          } else {
            allData.push([`=== ${data.sheetName} ===`]);
          }
          allData.push(...data.data);
          sheetNames.push(data.sheetName);
        }
      }

      setSheetData(allData);
      setCurrentSheetName(sheetNames.join(', '));
      setPhase('check');
      setLoading(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'データ取得エラー');
      setLoading(false);
    }
  };

  const runCheck = async () => {
    if (!sheetData) return;

    if (!getApiKey()) {
      setError('APIキーが設定されていません。メニュー → APIキー設定 から設定してください。');
      return;
    }

    setChecking(true);
    setError(null);
    setResult(null);

    try {
      // セパレータ行を除去してセル番地付きデータを生成
      const dataWithoutSeparators = sheetData.filter(row => !row[0]?.startsWith('==='));
      const cellAddressedData = generateCellAddressedDataFromArray(dataWithoutSeparators, 30, 26);

      console.log('[SpreadsheetChecker] Cell-addressed data:', cellAddressedData.slice(0, 500));

      // 工事情報を渡して妥当性判定
      const projectContext: ProjectContext = {
        contractor: contractor,
        projectName: projectInfo?.projectName,
        periodStart: projectInfo?.periodStart,
        periodEnd: projectInfo?.periodEnd,
        siteRepresentative: projectInfo?.siteRepresentative,
        today: projectInfo?.today || new Date().toISOString().slice(0, 10),
      };

      console.log('[SpreadsheetChecker] Project context:', projectContext);

      const extractionResult = await extractFields(cellAddressedData, projectContext);
      if (extractionResult) {
        console.log('[SpreadsheetChecker] Extraction result:', extractionResult);
        setResult(extractionResult);
      } else {
        setError('書類タイプの判定に失敗しました');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'チェックエラー');
    } finally {
      setChecking(false);
    }
  };

  const handleBack = () => {
    if (phase === 'check') {
      setPhase('select');
      setResult(null);
      setSheetData(null);
    } else {
      window.parent.postMessage({ type: 'spreadsheet-check-cancel' }, '*');
    }
  };

  const handleSaveAndBack = () => {
    if (result) {
      window.parent.postMessage({
        type: 'ai-check-result',
        result,
        contractor,
        contractorId,
        docType,
        docKey,
        spreadsheetId: spreadsheetId || fileId,
      }, '*');
    }
  };

  const fileTypeLabel = isExcel ? 'Excel' : 'スプレッドシート';

  // シート選択フェーズ
  if (phase === 'select') {
    return (
      <div className="ai-checker">
        <div className="checker-toolbar">
          <button className="back-btn" onClick={handleBack}>← 戻る</button>
          <span className="doc-info">
            {contractor} / {docType}
            <span className={`file-type-label ${isExcel ? 'excel' : 'sheet'}`}>{fileTypeLabel}</span>
          </span>
          <button
            className="check-btn"
            onClick={handleProceedToCheck}
            disabled={loading || selectedSheetIds.size === 0}
          >
            選択したシートをチェック ({selectedSheetIds.size}件)
          </button>
        </div>

        {error && <div className="error-message">{error}</div>}

        <div className="checker-content">
          <div className="preview-area spreadsheet-preview">
            {loading ? (
              <div className="loading-message">{fileTypeLabel}を読み込み中...</div>
            ) : sheetList.length > 0 ? (
              <div className="sheet-selector">
                <div className="sheet-selector-header">
                  <h3>{spreadsheetName}</h3>
                  <p>チェックするシートを選択してください</p>
                  <div className="debug-info">
                    <small>Build: {BUILD_VERSION}</small>
                  </div>
                </div>
                <div className="sheet-list">
                  {sheetList.map(sheet => (
                    <div
                      key={sheet.sheetId}
                      className={`sheet-item ${selectedSheetIds.has(sheet.sheetId) ? 'selected' : ''}`}
                      onClick={() => toggleSheetSelection(sheet.sheetId)}
                    >
                      <label className="sheet-checkbox">
                        <input
                          type="checkbox"
                          checked={selectedSheetIds.has(sheet.sheetId)}
                          onChange={() => toggleSheetSelection(sheet.sheetId)}
                        />
                        <span className="sheet-name">{sheet.name}</span>
                        <span className="sheet-size">({sheet.rowCount}行 × {sheet.colCount}列)</span>
                      </label>


                      {sheet.preview && sheet.preview.length > 0 && (
                        <div className="sheet-preview-mini">
                          <table>
                            <tbody>
                              {sheet.preview.map((row, ri) => (
                                <tr key={ri}>
                                  {row.map((cell, ci) => (
                                    <td key={ci}>{cell && cell.length > 15 ? cell.slice(0, 15) + '...' : cell || ''}</td>
                                  ))}
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="no-data-message">シートが見つかりません</div>
            )}
          </div>
        </div>
      </div>
    );
  }

  // チェック実行フェーズ
  return (
    <div className="ai-checker">
      <div className="checker-toolbar">
        <button className="back-btn" onClick={handleBack}>← シート選択に戻る</button>
        <span className="doc-info">
          {contractor} / {docType} - {currentSheetName}
          <span className={`file-type-label ${isExcel ? 'excel' : 'sheet'}`}>{fileTypeLabel}</span>
        </span>
        <button
          className="check-btn"
          onClick={runCheck}
          disabled={checking || loading || !sheetData}
        >
          {checking ? 'チェック中...' : 'AIチェック実行'}
        </button>
      </div>

      {error && <div className="error-message">{error}</div>}

      <div className="checker-content">
        <div className="preview-area spreadsheet-preview">
          {loading ? (
            <div className="loading-message">{fileTypeLabel}を読み込み中...</div>
          ) : sheetData ? (
            <div className="sheet-data-preview">
              <div className="sheet-info">
                対象シート: {currentSheetName} ({sheetData.length}行)
              </div>
              <div className="sheet-table-wrapper">
                <table className="sheet-table">
                  <tbody>
                    {sheetData.slice(0, 50).map((row, rowIndex) => (
                      <tr key={rowIndex} className={row[0]?.startsWith('===') ? 'sheet-separator' : ''}>
                        {row.slice(0, 15).map((cell, colIndex) => (
                          <td key={colIndex} title={cell}>
                            {cell && cell.length > 20 ? cell.slice(0, 20) + '...' : cell}
                          </td>
                        ))}
                        {row.length > 15 && <td>...</td>}
                      </tr>
                    ))}
                    {sheetData.length > 50 && (
                      <tr>
                        <td colSpan={Math.min(sheetData[0]?.length || 1, 16)}>... (以下省略)</td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          ) : (
            <div className="no-data-message">データがありません</div>
          )}
        </div>

        {result && (
          <div className="result-panel">
            <h3>書類判定結果</h3>
            <div className="document-type-badge">
              {result.documentType}
            </div>

            {/* 工事情報（照合元） */}
            {projectInfo && (
              <div className="project-info-section">
                <h4>照合元：工事情報</h4>
                <dl className="project-info-list">
                  {projectInfo.projectName && (
                    <>
                      <dt>工事名</dt>
                      <dd>{projectInfo.projectName}</dd>
                    </>
                  )}
                  <dt>元請受注者名</dt>
                  <dd>{contractor}</dd>
                  {projectInfo.siteRepresentative && (
                    <>
                      <dt>現場代理人</dt>
                      <dd>{projectInfo.siteRepresentative}</dd>
                    </>
                  )}
                  {(projectInfo.periodStart || projectInfo.periodEnd) && (
                    <>
                      <dt>工期</dt>
                      <dd>{projectInfo.periodStart} 〜 {projectInfo.periodEnd}</dd>
                    </>
                  )}
                </dl>
              </div>
            )}

            {/* 重要4項目 */}
            <div className="key-fields-section">
              <h4>重要項目</h4>
              <ul>
                {result.fields
                  .filter(f => KEY_FIELDS.includes(f.label))
                  .map((field, i) => (
                    <li key={i} className={`field-item key-field validation-${field.validation || 'unknown'}`}>
                      <div className="field-main">
                        <span className="validation-icon">
                          {field.validation === 'ok' ? '✓' : field.validation === 'warning' ? '⚠' : '✗'}
                        </span>
                        <span className="field-label">{field.label}</span>
                        <span className="field-value">
                          {field.value || <em className="not-found">未検出</em>}
                        </span>
                        {field.cell && <span className="field-cell">[{field.cell}]</span>}
                      </div>
                      {field.validationNote && (
                        <div className="validation-note">{field.validationNote}</div>
                      )}
                    </li>
                  ))}
              </ul>
            </div>

            {/* その他のフィールド */}
            {result.fields.filter(f => !KEY_FIELDS.includes(f.label)).length > 0 && (
              <div className="other-fields-section">
                <h4>その他</h4>
                <ul>
                  {result.fields
                    .filter(f => !KEY_FIELDS.includes(f.label))
                    .map((field, i) => (
                      <li key={i} className={`field-item validation-${field.validation || 'unknown'}`}>
                        <div className="field-main">
                          <span className="field-label">{field.label}</span>
                          <span className="field-value">
                            {field.value || <em className="not-found">未検出</em>}
                          </span>
                          {field.cell && <span className="field-cell">[{field.cell}]</span>}
                        </div>
                      </li>
                    ))}
                </ul>
              </div>
            )}

            <div className="result-actions">
              <button className="back-to-select-btn" onClick={handleBack}>
                ← シート選択に戻る
              </button>
              <button className="copy-json-btn" onClick={() => {
                const json = JSON.stringify({
                  documentType: result.documentType,
                  fields: result.fields.map(f => ({
                    label: f.label,
                    cell: f.cell,
                    value: f.value,
                  }))
                }, null, 2);
                navigator.clipboard.writeText(json);
                alert('JSONをコピーしました');
              }}>
                JSONコピー
              </button>
              <button className="save-btn" onClick={handleSaveAndBack}>
                保存して閉じる
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
