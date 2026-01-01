/**
 * PDF Viewer - GAS経由でPDFを取得して表示
 */
import { useState, useRef, useEffect } from 'react';
import { getDocument, GlobalWorkerOptions } from 'pdfjs-dist';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import { getCachedPdfAsync, setCachedPdf } from '../services/pdfCache';
import './PdfViewer.css';

GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url
).toString();

function getUrlParam(name: string): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get(name);
}

export function PdfViewer() {
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [pdfLoaded, setPdfLoaded] = useState(false);
  const [currentPage, setCurrentPage] = useState(1);
  const [totalPages, setTotalPages] = useState(0);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const pdfDocRef = useRef<PDFDocumentProxy | null>(null);

  const fileId = getUrlParam('fileId');
  const docType = getUrlParam('docType') || '書類';
  const contractor = getUrlParam('contractor') || '業者';

  // PDF読み込み
  useEffect(() => {
    if (!fileId) {
      setError('ファイルIDが指定されていません');
      setLoading(false);
      return;
    }

    const gasUrl = getUrlParam('gasUrl');

    const loadPdf = async () => {
      try {
        let pdfBytes: ArrayBuffer;

        // キャッシュをチェック（親ウィンドウのprefetchから、フェッチ中なら待機）
        const cached = await getCachedPdfAsync(fileId);
        if (cached) {
          console.log('[PdfViewer] PDF found in cache:', fileId);
          pdfBytes = cached;
        } else {
          // キャッシュになければGAS経由で取得
          if (!gasUrl) {
            setError('シート連携が未設定です。メニュー → シート連携設定 からGAS URLを設定してください。');
            setLoading(false);
            return;
          }
          console.log('[PdfViewer] PDF not in cache, fetching from GAS:', fileId);
          const response = await fetch(`${gasUrl}?action=fetchPdf&fileId=${fileId}`);
          if (!response.ok) throw new Error('PDF取得失敗');
          const data = await response.json();
          if (data.error) throw new Error(data.error);
          if (!data.base64) throw new Error('PDFデータがありません');
          // Base64をArrayBufferに変換
          const binary = atob(data.base64);
          const bytes = new Uint8Array(binary.length);
          for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
          }
          pdfBytes = bytes.buffer;
          // キャッシュに保存
          setCachedPdf(fileId, pdfBytes);
          console.log('[PdfViewer] PDF cached:', fileId);
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

  const handleCheck = () => {
    window.parent.postMessage({ type: 'viewer-check' }, '*');
  };

  return (
    <div className="pdf-viewer">
      <div className="viewer-toolbar">
        <button className="back-btn" onClick={handleBack}>← 戻る</button>
        <span className="doc-info">{contractor} / {docType}</span>
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
          <button className="check-btn" onClick={handleCheck} disabled={loading}>
            AIチェック
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
      </div>
    </div>
  );
}
