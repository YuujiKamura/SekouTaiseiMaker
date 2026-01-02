/**
 * Spreadsheet AI Checker - スプレッドシートのAIチェック
 *
 * ## 変更履歴
 * - 2026-01-02: シート選択UI追加（複数シートから対象を選択可能に）
 * - 2026-01-02: 初期実装
 */
import { useState, useEffect } from 'react';
import { checkSpreadsheet, type CheckResult } from '../services/gemini';
import { getApiKey } from '../services/apiKey';
import './AiChecker.css';

interface SheetInfo {
  sheetId: number;
  name: string;
  rowCount: number;
  colCount: number;
  preview: string[][];
}

interface SheetListResponse {
  success?: boolean;
  error?: string;
  spreadsheetId: string;
  spreadsheetName: string;
  sheets: SheetInfo[];
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

  const spreadsheetId = getUrlParam('spreadsheetId');
  const gid = getUrlParam('gid');
  const docType = getUrlParam('docType') || '書類';
  const contractor = getUrlParam('contractor') || '業者';
  const gasUrl = getUrlParam('gasUrl');
  const contractorId = getUrlParam('contractorId') || '';
  const docKey = getUrlParam('docKey') || docType;

  // シート一覧を取得
  useEffect(() => {
    if (!spreadsheetId) {
      setError('スプレッドシートIDが指定されていません');
      setLoading(false);
      return;
    }

    if (!gasUrl) {
      setError('GAS URLが設定されていません');
      setLoading(false);
      return;
    }

    const fetchSheetList = async () => {
      try {
        const url = `${gasUrl}?action=listSheets&spreadsheetId=${encodeURIComponent(spreadsheetId)}`;
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

    fetchSheetList();
  }, [spreadsheetId, gid, gasUrl]);

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
      // 選択されたシートのデータを取得（複数の場合は結合）
      const allData: string[][] = [];
      const sheetNames: string[] = [];

      for (const sheetId of selectedSheetIds) {
        const url = `${gasUrl}?action=fetchSpreadsheet&spreadsheetId=${encodeURIComponent(spreadsheetId!)}&gid=${sheetId}`;
        const response = await fetch(url, { cache: 'no-store' });
        const data = await response.json();

        if (data.error) {
          throw new Error(data.error);
        }

        // シート名を区切りとして追加
        if (allData.length > 0) {
          allData.push(['']); // 空行で区切り
          allData.push([`=== ${data.sheetName} ===`]);
        } else {
          allData.push([`=== ${data.sheetName} ===`]);
        }
        allData.push(...data.data);
        sheetNames.push(data.sheetName);
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
        spreadsheetId,
      }, '*');
    }
  };

  // シート選択フェーズ
  if (phase === 'select') {
    return (
      <div className="ai-checker">
        <div className="checker-toolbar">
          <button className="back-btn" onClick={handleBack}>← 戻る</button>
          <span className="doc-info">{contractor} / {docType}</span>
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
              <div className="loading-message">シート一覧を読み込み中...</div>
            ) : sheetList.length > 0 ? (
              <div className="sheet-selector">
                <div className="sheet-selector-header">
                  <h3>{spreadsheetName}</h3>
                  <p>AIチェックを実行するシートを選択してください（複数選択可）</p>
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
        <span className="doc-info">{contractor} / {docType} - {currentSheetName}</span>
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
            <div className="loading-message">スプレッドシートを読み込み中...</div>
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
                            {cell.length > 20 ? cell.slice(0, 20) + '...' : cell}
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
