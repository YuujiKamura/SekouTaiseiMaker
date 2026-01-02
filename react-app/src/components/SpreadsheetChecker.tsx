/**
 * Spreadsheet AI Checker - スプレッドシート/ExcelのAIチェック
 *
 * ## 変更履歴
 * - 2026-01-02: 重要フィールド表示・事業所名バリデーション追加
 * - 2026-01-02: Excelファイル対応（SheetJSでパース）
 * - 2026-01-02: シート選択UI追加（複数シートから対象を選択可能に）
 * - 2026-01-02: 初期実装
 */
import { useState, useEffect } from 'react';
import { checkSpreadsheet, type CheckResult } from '../services/gemini';
import { getApiKey } from '../services/apiKey';
import { safeBase64ToArrayBuffer } from '../utils/base64';
import * as XLSX from 'xlsx';
import './AiChecker.css';

// 抽出した重要フィールド
interface ExtractedFields {
  officeName: string | null;      // 事業所名
  directorName: string | null;    // 所長名
  createdDate: string | null;     // 名簿作成日
  submittedDate: string | null;   // 提出日
  projectName: string | null;     // 工事名（検証用）
}

interface SheetInfo {
  sheetId: number;
  name: string;
  rowCount: number;
  colCount: number;
  preview: string[][];
  fields?: ExtractedFields;
}

interface SheetListResponse {
  success?: boolean;
  error?: string;
  spreadsheetId: string;
  spreadsheetName: string;
  sheets: SheetInfo[];
}

// 施工体制台帳から重要フィールドを抽出（フロントエンド版）
function extractImportantFields(data: string[][]): ExtractedFields {
  const fields: ExtractedFields = {
    officeName: null,
    directorName: null,
    createdDate: null,
    submittedDate: null,
    projectName: null
  };

  if (!data || data.length === 0) return fields;

  for (let row = 0; row < data.length; row++) {
    for (let col = 0; col < (data[row]?.length || 0); col++) {
      const cell = String(data[row][col] || '').trim();

      // 事業所名・事業所の名称
      if ((cell.includes('事業所') || cell.includes('事業所の名称')) && !fields.officeName) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < (data[row + 1]?.length || 0) ? data[row + 1][col] : null;
        fields.officeName = rightValue || belowValue || null;
      }

      // 所長・現場代理人・責任者
      if ((cell.includes('所長') || cell.includes('現場代理人') || cell.includes('責任者')) && !fields.directorName) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < (data[row + 1]?.length || 0) ? data[row + 1][col] : null;
        fields.directorName = rightValue || belowValue || null;
      }

      // 作成日
      if ((cell.includes('作成') && (cell.includes('日') || cell.includes('年月'))) && !fields.createdDate) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < (data[row + 1]?.length || 0) ? data[row + 1][col] : null;
        fields.createdDate = rightValue || belowValue || null;
      }

      // 提出日
      if (cell.includes('提出') && !fields.submittedDate) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < (data[row + 1]?.length || 0) ? data[row + 1][col] : null;
        fields.submittedDate = rightValue || belowValue || null;
      }

      // 工事名称・工事名
      if ((cell === '工事名称' || cell === '工事名') && !fields.projectName) {
        const rightValue = col + 1 < data[row].length ? data[row][col + 1] : null;
        const belowValue = row + 1 < data.length && col < (data[row + 1]?.length || 0) ? data[row + 1][col] : null;
        fields.projectName = rightValue || belowValue || null;
      }
    }
  }

  return fields;
}

// 事業所名のバリデーション
interface OfficeValidation {
  isValid: boolean;
  warning: string | null;
}

function validateOfficeName(officeName: string | null, currentProjectName: string | null): OfficeValidation {
  if (!officeName) {
    return { isValid: false, warning: '事業所名が未入力です' };
  }

  // 空白や「-」のみの場合
  if (officeName.trim() === '' || officeName === '-') {
    return { isValid: false, warning: '事業所名が未入力です' };
  }

  // 許可されるパターン：
  // 1. 会社名（本社、支店など）
  // 2. 工事事務所名
  // 3. 現在の工事名

  // 現在の工事名と一致する場合はOK（例外許可）
  if (currentProjectName && officeName.includes(currentProjectName)) {
    return { isValid: true, warning: null };
  }

  // 工事事務所パターン
  if (officeName.includes('事務所') || officeName.includes('工事所')) {
    return { isValid: true, warning: null };
  }

  // 会社関連パターン
  if (officeName.includes('本社') || officeName.includes('支店') || officeName.includes('営業所') || officeName.includes('株式会社') || officeName.includes('有限会社')) {
    return { isValid: true, warning: null };
  }

  // 「工事」を含むが現在の工事名でない場合は警告
  if (officeName.includes('工事')) {
    return { isValid: true, warning: '他の工事名が入っている可能性があります' };
  }

  return { isValid: true, warning: null };
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
  const [result, setResult] = useState<CheckResult | null>(null);
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

  const spreadsheetId = getUrlParam('spreadsheetId');
  const fileId = getUrlParam('fileId');
  const isExcel = getUrlParam('isExcel') === 'true';
  const gid = getUrlParam('gid');
  const docType = getUrlParam('docType') || '書類';
  const contractor = getUrlParam('contractor') || '業者';
  const gasUrl = getUrlParam('gasUrl');
  const contractorId = getUrlParam('contractorId') || '';
  const docKey = getUrlParam('docKey') || docType;
  const projectName = getUrlParam('projectName') || ''; // 現在の工事名（バリデーション用）

  // データ取得（Excel or スプレッドシート）
  useEffect(() => {
    if (!gasUrl) {
      setError('GAS URLが設定されていません');
      setLoading(false);
      return;
    }

    if (isExcel && fileId) {
      // Excelファイルを取得してSheetJSでパース
      fetchExcelFile();
    } else if (spreadsheetId) {
      // Google スプレッドシートのシート一覧を取得
      fetchSheetList();
    } else {
      setError('ファイルIDが指定されていません');
      setLoading(false);
    }
  }, [spreadsheetId, fileId, isExcel, gasUrl]);

  const fetchExcelFile = async () => {
    try {
      const url = `${gasUrl}?action=fetchExcelAsBase64&fileId=${encodeURIComponent(fileId!)}`;
      const response = await fetch(url, { cache: 'no-store' });
      const data: ExcelResponse = await response.json();

      if (data.error) {
        throw new Error(data.error);
      }

      // Base64をデコードしてSheetJSでパース（sanitization付き）
      const bytes = new Uint8Array(safeBase64ToArrayBuffer(data.base64));
      const workbook = XLSX.read(bytes, { type: 'array' });
      setExcelWorkbook(workbook);
      setSpreadsheetName(data.fileName);

      // シート一覧を構築
      const sheets: SheetInfo[] = workbook.SheetNames.map((name, index) => {
        const sheet = workbook.Sheets[name];
        const jsonData = XLSX.utils.sheet_to_json<string[]>(sheet, { header: 1 }) as string[][];
        const rowCount = jsonData.length;
        const colCount = jsonData[0]?.length || 0;

        // フィールド抽出用に10行×20列を取得
        const extendedData = jsonData.slice(0, 10).map(row =>
          (row || []).slice(0, 20).map(cell => String(cell ?? ''))
        );
        const fields = extractImportantFields(extendedData);

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
          fields
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

      // gidが指定されていれば、そのシートを初期選択
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
        // Excelの場合はローカルでパース
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
        // スプレッドシートの場合はGAS経由
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
      const checkResult = await checkSpreadsheet(sheetData, docType, contractor);
      setResult(checkResult);
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

  // ファイルタイプ表示
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
                  <p>AIチェックを実行するシートを選択してください（複数選択可）</p>
                </div>
                <div className="sheet-list">
                  {sheetList.map(sheet => {
                    const fields = sheet.fields;
                    const validation = fields ? validateOfficeName(fields.officeName, projectName || null) : null;
                    const hasWarning = validation?.warning;

                    return (
                      <div
                        key={sheet.sheetId}
                        className={`sheet-item ${selectedSheetIds.has(sheet.sheetId) ? 'selected' : ''} ${hasWarning ? 'has-warning' : ''}`}
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

                        {/* 抽出したフィールド情報 */}
                        {fields && (fields.officeName || fields.directorName || fields.createdDate || fields.submittedDate) && (
                          <div className="sheet-fields">
                            {fields.officeName && (
                              <div className={`field-row ${hasWarning ? 'warning' : ''}`}>
                                <span className="field-label">事業所:</span>
                                <span className="field-value">{fields.officeName}</span>
                                {hasWarning && <span className="field-warning">⚠ {validation.warning}</span>}
                              </div>
                            )}
                            {fields.directorName && (
                              <div className="field-row">
                                <span className="field-label">所長:</span>
                                <span className="field-value">{fields.directorName}</span>
                              </div>
                            )}
                            {fields.createdDate && (
                              <div className="field-row">
                                <span className="field-label">作成日:</span>
                                <span className="field-value">{fields.createdDate}</span>
                              </div>
                            )}
                            {fields.submittedDate && (
                              <div className="field-row">
                                <span className="field-label">提出日:</span>
                                <span className="field-value">{fields.submittedDate}</span>
                              </div>
                            )}
                          </div>
                        )}

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
                    );
                  })}
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
          <div className={`result-panel status-${result.status}`}>
            <h3>チェック結果</h3>
            <div className={`status-badge ${result.status}`}>
              {result.status === 'ok' ? '✓ OK' : result.status === 'warning' ? '⚠ 要確認' : '✗ エラー'}
            </div>
            <p className="summary">{result.summary}</p>

            {result.items.length > 0 && (
              <div className="items">
                <h4>詳細</h4>
                <ul>
                  {result.items.map((item, i) => (
                    <li key={i} className={`item-${item.type}`}>
                      <span className="icon">
                        {item.type === 'ok' ? '✓' : item.type === 'warning' ? '⚠' : '✗'}
                      </span>
                      {item.message}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {result.missing_fields.length > 0 && (
              <div className="missing-fields">
                <h4>未記入項目</h4>
                <ul>
                  {result.missing_fields.map((field, i) => (
                    <li key={i}>
                      <strong>{field.field}</strong>
                      <span className="location">({field.location})</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            <div className="result-actions">
              <button className="save-btn" onClick={handleSaveAndBack}>
                保存して戻る
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
