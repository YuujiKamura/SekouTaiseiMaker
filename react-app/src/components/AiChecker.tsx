/**
 * AI Document Checker - Gemini APIで書類をチェック
 */
import { useState, useRef, useEffect } from 'react';
import { getDocument, GlobalWorkerOptions } from 'pdfjs-dist';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import { checkDocumentImage, type CheckResult } from '../services/gemini';
import { getApiKey } from '../services/apiKey';
import { getCachedPdfAsync, setCachedPdf, isCacheValid, invalidateCache } from '../services/pdfCache';
import { safeBase64ToArrayBuffer } from '../utils/base64';
import './AiChecker.css';

GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url
).toString();

function getUrlParam(name: string): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get(name);
}

export function AiChecker() {
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<CheckResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pdfLoaded, setPdfLoaded] = useState(false);
  const [currentPage, setCurrentPage] = useState(1);
  const [totalPages, setTotalPages] = useState(0);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const pdfDocRef = useRef<PDFDocumentProxy | null>(null);

  const fileId = getUrlParam('fileId');
  const docType = getUrlParam('docType') || '書類';
  const contractor = getUrlParam('contractor') || '業者';
  const docKey = getUrlParam('docKey') || docType;  // doc_key（07_建退共など）
  const contractorId = getUrlParam('contractorId') || '';

  // PDF読み込み
  useEffect(() => {
    if (!fileId) {
      setError('ファイルIDが指定されていません');
      return;
    }

    const gasUrl = getUrlParam('gasUrl');

    const loadPdf = async () => {
      try {
        let pdfBytes: ArrayBuffer | undefined;
        let modifiedTime: string | undefined;

        // GAS URLが必要
        if (!gasUrl) {
          setError('シート連携が未設定です。メニュー → シート連携設定 からGAS URLを設定してください。');
          return;
        }

        // GASから最新ファイル情報を取得（フォルダ内の同名or最新ファイルを探す）
        let actualFileId = fileId;
        try {
          const infoRes = await fetch(`${gasUrl}?action=getLatestFile&fileId=${fileId}`, { cache: 'no-store' });
          const info = await infoRes.json();
          console.log('[AiChecker] GAS getLatestFile response:', info);
          if (!info.error) {
            modifiedTime = info.modifiedTime;
            // ファイルIDが更新された場合は新しいIDを使用
            if (info.wasUpdated && info.fileId) {
              console.log('[AiChecker] File updated:', fileId, '->', info.fileId);
              actualFileId = info.fileId;
              // スプレッドシートのURLを更新（GETリクエスト）
              if (contractorId && docKey) {
                try {
                  const updateUrl = `${gasUrl}?action=updateDocUrl&contractorId=${encodeURIComponent(contractorId)}&docKey=${encodeURIComponent(docKey)}&newFileId=${encodeURIComponent(info.fileId)}`;
                  await fetch(updateUrl, { cache: 'no-store' });
                  console.log('[AiChecker] Spreadsheet URL updated');
                } catch (e) {
                  console.error('[AiChecker] Failed to update spreadsheet URL:', e);
                }
              }
            }
          }
        } catch {
          // ファイル情報取得失敗は無視
        }

        // キャッシュの有効性をチェック（actualFileIdを使用）
        let useCache = false;
        if (modifiedTime) {
          useCache = await isCacheValid(actualFileId, modifiedTime);
          if (!useCache) {
            await invalidateCache(actualFileId);
            console.log('[AiChecker] Cache invalidated: file was modified');
          }
        }

        if (useCache) {
          const cached = await getCachedPdfAsync(actualFileId);
          if (cached) {
            console.log('[AiChecker] PDF found in valid cache:', actualFileId);
            pdfBytes = cached;
          }
        }

        if (!pdfBytes) {
          console.log('[AiChecker] Fetching PDF from GAS:', actualFileId);
          // GAS経由でPDFを取得（actualFileIdを使用）
          const response = await fetch(`${gasUrl}?action=fetchPdf&fileId=${actualFileId}`, { cache: 'no-store' });
          if (!response.ok) throw new Error('PDF取得失敗');
          const data = await response.json();
          if (data.error) throw new Error(data.error);
          if (!data.base64) throw new Error('PDFデータがありません');
          // Base64をArrayBufferに変換（sanitization付き）
          pdfBytes = safeBase64ToArrayBuffer(data.base64);
          // キャッシュに保存（modifiedTime付き）
          await setCachedPdf(actualFileId, pdfBytes, modifiedTime || data.modifiedTime);
          console.log('[AiChecker] PDF cached:', actualFileId);
        }

        const pdf = await getDocument({ data: pdfBytes }).promise;
        pdfDocRef.current = pdf;
        setTotalPages(pdf.numPages);
        setPdfLoaded(true);
        renderPage(1);
      } catch (e) {
        setError(e instanceof Error ? e.message : 'PDF読み込みエラー');
      }
    };

    loadPdf();
  }, [fileId]);

  const renderPage = async (pageNum: number) => {
    const pdf = pdfDocRef.current;
    const canvas = canvasRef.current;
    if (!pdf || !canvas) return;

    const page = await pdf.getPage(pageNum);
    const viewport = page.getViewport({ scale: 1.5 });
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    canvas.width = viewport.width;
    canvas.height = viewport.height;

    await page.render({ canvasContext: ctx, viewport, canvas }).promise;
    setCurrentPage(pageNum);
  };

  const runCheck = async () => {
    const canvas = canvasRef.current;
    if (!canvas || !pdfLoaded) return;

    if (!getApiKey()) {
      setError('APIキーが設定されていません。メニュー → APIキー設定 から設定してください。');
      return;
    }

    setChecking(true);
    setError(null);
    setResult(null);

    try {
      // キャンバスをBase64に変換
      const dataUrl = canvas.toDataURL('image/png');
      const base64 = dataUrl.split(',')[1];

      const checkResult = await checkDocumentImage(base64, 'image/png', docType, contractor);
      setResult(checkResult);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'チェックエラー');
    } finally {
      setChecking(false);
    }
  };

  const handleBack = () => {
    window.parent.postMessage({ type: 'ai-check-cancel' }, '*');
  };

  const handleSaveAndBack = () => {
    if (result) {
      window.parent.postMessage({
        type: 'ai-check-result',
        result,
        contractor,
        contractorId,
        docType,
        docKey,  // 実際のドキュメントキー（07_建退共など）
        fileId,
      }, '*');
    }
  };

  return (
    <div className="ai-checker">
      <div className="checker-toolbar">
        <button className="back-btn" onClick={handleBack}>← 戻る</button>
        <span className="doc-info">{contractor} / {docType}</span>
        <div className="page-nav">
          <button
            onClick={() => renderPage(currentPage - 1)}
            disabled={currentPage <= 1}
          >◀</button>
          <span>{currentPage} / {totalPages}</span>
          <button
            onClick={() => renderPage(currentPage + 1)}
            disabled={currentPage >= totalPages}
          >▶</button>
        </div>
        <button
          className="check-btn"
          onClick={runCheck}
          disabled={checking || !pdfLoaded}
        >
          {checking ? 'チェック中...' : 'AIチェック実行'}
        </button>
      </div>

      {error && <div className="error-message">{error}</div>}

      <div className="checker-content">
        <div className="preview-area">
          <canvas ref={canvasRef} />
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
