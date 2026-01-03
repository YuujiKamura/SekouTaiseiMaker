/**
 * PDF Viewer - GAS経由でPDFを取得して表示 + インラインAIチェック
 */
import { useState, useRef, useEffect } from 'react';
import { getDocument, GlobalWorkerOptions } from 'pdfjs-dist';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import { getCachedPdfAsync, setCachedPdf, isCacheValid, invalidateCache } from '../services/pdfCache';
import { checkDocumentImage, type CheckResult } from '../services/gemini';
import { getApiKey } from '../services/apiKey';
import { safeBase64ToArrayBuffer } from '../utils/base64';
import './PdfViewer.css';

GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url
).toString();

function getUrlParam(name: string): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get(name);
}

/** フィールドキーを日本語ラベルに変換 */
function formatFieldName(key: string): string {
  const labels: Record<string, string> = {
    representative_name: '現場代理人名',
    chief_engineer_name: '主任技術者名',
    qualification_number: '資格番号',
  };
  return labels[key] || key;
}

export function PdfViewer() {
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [pdfLoaded, setPdfLoaded] = useState(false);
  const [currentPage, setCurrentPage] = useState(1);
  const [totalPages, setTotalPages] = useState(0);
  const [fileModifiedTime, setFileModifiedTime] = useState<string | null>(null);

  // AIチェック用のstate
  const [checking, setChecking] = useState(false);
  const [checkResult, setCheckResult] = useState<CheckResult | null>(null);
  const [showResult, setShowResult] = useState(false);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const pdfDocRef = useRef<PDFDocumentProxy | null>(null);

  const fileId = getUrlParam('fileId');
  const docType = getUrlParam('docType') || '書類';
  const contractor = getUrlParam('contractor') || '業者';
  const contractorId = getUrlParam('contractorId') || '';
  const docKey = getUrlParam('docKey') || '';
  const gasUrl = getUrlParam('gasUrl');

  // PDF読み込み
  useEffect(() => {
    if (!fileId) {
      setError('ファイルIDが指定されていません');
      setLoading(false);
      return;
    }

    const loadPdf = async () => {
      try {
        let pdfBytes: ArrayBuffer | undefined;
        let modifiedTime: string | undefined;

        // GAS URLが必要
        if (!gasUrl) {
          setError('シート連携が未設定です。メニュー → シート連携設定 からGAS URLを設定してください。');
          setLoading(false);
          return;
        }

        // GASから最新ファイル情報を取得（フォルダ内の同名or最新ファイルを探す）
        let actualFileId = fileId;
        try {
          // まずファイル情報を取得してMIMEタイプをチェック
          const fileInfoRes = await fetch(`${gasUrl}?action=getFileInfo&fileId=${fileId}`, { cache: 'no-store' });
          const fileInfo = await fileInfoRes.json();
          console.log('[PdfViewer] File info:', fileInfo);

          // MIMEタイプチェック（Excel/Spreadsheetの場合はエラー）
          if (fileInfo.mimeType) {
            const mime = fileInfo.mimeType.toLowerCase();
            if (mime.includes('spreadsheet') || mime.includes('excel') || mime.includes('ms-excel')) {
              throw new Error('このファイルはExcel/スプレッドシート形式です。一覧に戻って正しいビューワで開いてください。');
            }
          }

          const infoRes = await fetch(`${gasUrl}?action=getLatestFile&fileId=${fileId}`, { cache: 'no-store' });
          const info = await infoRes.json();
          console.log('[PdfViewer] GAS getLatestFile response:', info);
          if (!info.error) {
            modifiedTime = info.modifiedTime;
            setFileModifiedTime(modifiedTime || null);
            // ファイルIDが更新された場合は新しいIDを使用
            if (info.wasUpdated && info.fileId) {
              console.log('[PdfViewer] File updated:', fileId, '->', info.fileId);
              actualFileId = info.fileId;
              // スプレッドシートのURLを更新（GETリクエスト）
              if (contractorId && docKey) {
                try {
                  const updateUrl = `${gasUrl}?action=updateDocUrl&contractorId=${encodeURIComponent(contractorId)}&docKey=${encodeURIComponent(docKey)}&newFileId=${encodeURIComponent(info.fileId)}`;
                  await fetch(updateUrl, { cache: 'no-store' });
                  console.log('[PdfViewer] Spreadsheet URL updated');
                } catch (e) {
                  console.error('[PdfViewer] Failed to update spreadsheet URL:', e);
                }
              }
            }
          }
        } catch (e) {
          if (e instanceof Error && e.message.includes('Excel')) {
            throw e; // Excel関連のエラーはそのままスロー
          }
          console.error('[PdfViewer] getLatestFile failed:', e);
        }

        // キャッシュの有効性をチェック（actualFileIdを使用）
        let useCache = false;
        console.log('[PdfViewer] modifiedTime from GAS:', modifiedTime, 'actualFileId:', actualFileId);
        if (modifiedTime) {
          useCache = await isCacheValid(actualFileId, modifiedTime);
          console.log('[PdfViewer] Cache valid:', useCache);
          if (!useCache) {
            await invalidateCache(actualFileId);
            console.log('[PdfViewer] Cache invalidated: file was modified');
          }
        } else {
          // modifiedTimeが取得できない場合はキャッシュを使わない
          console.log('[PdfViewer] No modifiedTime, skipping cache');
          await invalidateCache(actualFileId);
        }

        if (useCache) {
          const cached = await getCachedPdfAsync(actualFileId);
          if (cached) {
            console.log('[PdfViewer] PDF found in valid cache:', actualFileId);
            pdfBytes = cached;
          }
        }

        if (!pdfBytes) {
          // GAS経由で取得（actualFileIdを使用）
          console.log('[PdfViewer] Fetching PDF from GAS:', actualFileId);
          const response = await fetch(`${gasUrl}?action=fetchPdf&fileId=${actualFileId}`, { cache: 'no-store' });
          if (!response.ok) throw new Error('PDF取得失敗');
          const data = await response.json();
          if (data.error) {
            // ファイルタイプエラーの場合は分かりやすいメッセージ
            if (data.error.includes('Unsupported file type')) {
              const mimeType = data.error.split(': ')[1] || '';
              if (mimeType.includes('spreadsheet') || mimeType.includes('excel')) {
                throw new Error('このファイルはExcel形式です。シートビューワで開いてください。');
              } else if (mimeType.includes('image')) {
                throw new Error('このファイルは画像形式です。');
              } else {
                throw new Error(`非対応のファイル形式です: ${mimeType}`);
              }
            }
            throw new Error(data.error);
          }
          if (!data.base64) throw new Error('PDFデータがありません');
          // Base64をArrayBufferに変換（sanitization付き）
          pdfBytes = safeBase64ToArrayBuffer(data.base64);
          // キャッシュに保存（modifiedTime付き）
          await setCachedPdf(actualFileId, pdfBytes, modifiedTime || data.modifiedTime);
          console.log('[PdfViewer] PDF cached:', actualFileId);
        }

        const pdf = await getDocument({ data: pdfBytes }).promise;
        pdfDocRef.current = pdf;
        setTotalPages(pdf.numPages);
        setLoading(false);
        setPdfLoaded(true);
      } catch (e) {
        setError(e instanceof Error ? e.message : 'PDF読み込みエラー');
        setLoading(false);
      }
    };

    loadPdf();
  }, [fileId]);

  // PDF読み込み完了後に最初のページを描画
  useEffect(() => {
    if (pdfLoaded && canvasRef.current) {
      renderPage(1);
    }
  }, [pdfLoaded]);

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

  const handleBack = () => {
    window.parent.postMessage({ type: 'viewer-back' }, '*');
  };

  const handleEdit = () => {
    window.parent.postMessage({ type: 'viewer-edit' }, '*');
  };

  // インラインAIチェック実行
  const handleCheck = async () => {
    const canvas = canvasRef.current;
    if (!canvas || !pdfLoaded) return;

    if (!getApiKey()) {
      setError('APIキーが設定されていません。メニュー → APIキー設定 から設定してください。');
      return;
    }

    setChecking(true);
    setError(null);

    try {
      // キャンバスをBase64に変換
      const dataUrl = canvas.toDataURL('image/png');
      const base64 = dataUrl.split(',')[1];

      const result = await checkDocumentImage(base64, 'image/png', docType, contractor);
      setCheckResult(result);
      setShowResult(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'チェックエラー');
    } finally {
      setChecking(false);
    }
  };

  // 結果を保存して閉じる
  const handleSaveResult = () => {
    if (checkResult) {
      window.parent.postMessage({
        type: 'ai-check-result',
        result: checkResult,
        contractor,
        contractorId,
        docType,
        docKey,
        fileId,
      }, '*');
    }
    setShowResult(false);
  };

  // 結果パネルを閉じる（保存せず）
  const handleCloseResult = () => {
    setShowResult(false);
  };

  const handleForceReload = async () => {
    if (!fileId) return;
    setLoading(true);
    await invalidateCache(fileId);
    location.reload();
  };

  // ファイル更新日時をフォーマット
  const formatModifiedTime = (isoString: string | null): string => {
    if (!isoString) return '';
    try {
      const date = new Date(isoString);
      return date.toLocaleString('ja-JP', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit'
      });
    } catch {
      return isoString;
    }
  };

  const driveUrl = fileId ? `https://drive.google.com/file/d/${fileId}/view` : '';

  return (
    <div className="pdf-viewer">
      <div className="viewer-toolbar">
        <button className="back-btn" onClick={handleBack}>← 戻る</button>
        <span className="doc-info">{contractor} / {docType}</span>
        {fileModifiedTime && (
          <span className="file-modified-time" title={driveUrl}>
            更新: {formatModifiedTime(fileModifiedTime)}
          </span>
        )}
        <div className="page-nav">
          <button
            onClick={() => renderPage(currentPage - 1)}
            disabled={currentPage <= 1 || loading}
          >◀</button>
          <span>{currentPage} / {totalPages}</span>
          <button
            onClick={() => renderPage(currentPage + 1)}
            disabled={currentPage >= totalPages || loading}
          >▶</button>
        </div>
        <div className="toolbar-actions">
          <button className="edit-btn" onClick={handleEdit} disabled={loading}>
            編集
          </button>
          <button className="check-btn" onClick={handleCheck} disabled={loading || checking}>
            {checking ? 'チェック中...' : 'AIチェック'}
          </button>
          <button className="reload-btn" onClick={handleForceReload} disabled={loading} title="キャッシュを無視して再読み込み">
            🔄
          </button>
        </div>
      </div>

      {error && <div className="error-message">{error}</div>}

      <div className="viewer-content">
        {loading ? (
          <div className="loading">
            <div className="loading-spinner"></div>
            <div className="loading-text">PDF読み込み中</div>
          </div>
        ) : (
          <canvas ref={canvasRef} />
        )}

        {/* インラインチェック結果パネル */}
        {showResult && checkResult && (
          <div className={`inline-result-panel status-${checkResult.status}`}>
            <div className="result-header">
              <h3>チェック結果: {docType}</h3>
              <button className="close-btn" onClick={handleCloseResult}>×</button>
            </div>

            <div className={`status-badge ${checkResult.status}`}>
              {checkResult.status === 'ok' ? '✓ OK' : checkResult.status === 'warning' ? '⚠ 要確認' : '✗ エラー'}
            </div>

            {/* 抽出フィールド（書類タイプごとの必須項目） */}
            {checkResult.extracted_fields && Object.keys(checkResult.extracted_fields).length > 0 && (
              <div className="extracted-fields">
                <h4>抽出データ</h4>
                <dl className="field-list">
                  {Object.entries(checkResult.extracted_fields).map(([key, value]) => (
                    <div key={key} className="field-item">
                      <dt>{formatFieldName(key)}</dt>
                      <dd>{value}</dd>
                    </div>
                  ))}
                </dl>
              </div>
            )}

            <p className="summary">{checkResult.summary}</p>

            <div className="result-actions">
              <button className="save-btn" onClick={handleSaveResult}>
                保存して閉じる
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
