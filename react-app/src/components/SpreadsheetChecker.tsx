/**
 * Spreadsheet AI Checker - スプレッドシートのAIチェック
 */
import { useState, useEffect } from 'react';
import { checkSpreadsheet, type CheckResult } from '../services/gemini';
import { getApiKey } from '../services/apiKey';
import './AiChecker.css';

function getUrlParam(name: string): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get(name);
}

export function SpreadsheetChecker() {
  const [loading, setLoading] = useState(true);
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<CheckResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sheetData, setSheetData] = useState<string[][] | null>(null);
  const [sheetInfo, setSheetInfo] = useState<{
    sheetName: string;
    rowCount: number;
    colCount: number;
  } | null>(null);

  const spreadsheetId = getUrlParam('spreadsheetId');
  const gid = getUrlParam('gid');
  const docType = getUrlParam('docType') || '書類';
  const contractor = getUrlParam('contractor') || '業者';
  const gasUrl = getUrlParam('gasUrl');
  const contractorId = getUrlParam('contractorId') || '';
  const docKey = getUrlParam('docKey') || docType;

  // スプレッドシートデータを取得
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

    const fetchData = async () => {
      try {
        let url = `${gasUrl}?action=fetchSpreadsheet&spreadsheetId=${encodeURIComponent(spreadsheetId)}`;
        if (gid) {
          url += `&gid=${encodeURIComponent(gid)}`;
        }

        const response = await fetch(url, { cache: 'no-store' });
        const data = await response.json();

        if (data.error) {
          throw new Error(data.error);
        }

        setSheetData(data.data);
        setSheetInfo({
          sheetName: data.sheetName,
          rowCount: data.rowCount,
          colCount: data.colCount,
        });
        setLoading(false);
      } catch (e) {
        setError(e instanceof Error ? e.message : 'データ取得エラー');
        setLoading(false);
      }
    };

    fetchData();
  }, [spreadsheetId, gid, gasUrl]);

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
    window.parent.postMessage({ type: 'spreadsheet-check-cancel' }, '*');
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

  return (
    <div className="ai-checker">
      <div className="checker-toolbar">
        <button className="back-btn" onClick={handleBack}>← 戻る</button>
        <span className="doc-info">{contractor} / {docType}</span>
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
                シート名: {sheetInfo?.sheetName} ({sheetInfo?.rowCount}行 × {sheetInfo?.colCount}列)
              </div>
              <div className="sheet-table-wrapper">
                <table className="sheet-table">
                  <tbody>
                    {sheetData.slice(0, 30).map((row, rowIndex) => (
                      <tr key={rowIndex}>
                        {row.slice(0, 15).map((cell, colIndex) => (
                          <td key={colIndex} title={cell}>
                            {cell.length > 20 ? cell.slice(0, 20) + '...' : cell}
                          </td>
                        ))}
                        {row.length > 15 && <td>...</td>}
                      </tr>
                    ))}
                    {sheetData.length > 30 && (
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
