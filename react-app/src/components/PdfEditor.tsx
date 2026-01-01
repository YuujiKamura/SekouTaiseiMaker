import { useState, useRef, useEffect, useCallback } from 'react';
import { PDFDocument, rgb, degrees } from 'pdf-lib';
import fontkit from '@pdf-lib/fontkit';
import { getDocument, GlobalWorkerOptions, AnnotationMode } from 'pdfjs-dist';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import './PdfEditor.css';

// PDF.js worker設定 (v5.x - use bundled worker)
GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url
).toString();

interface TextAnnotation {
  id: string;
  x: number;
  y: number;
  text: string;
  fontSize: number;
  fontFamily: 'mincho' | 'gothic';
  color: string;
  page: number;
}

interface PdfEditorProps {
  pdfUrl?: string;
  onSave?: (pdfBytes: Uint8Array) => void;
}

// URL params helper
function getUrlParam(name: string): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get(name);
}

export function PdfEditor({ pdfUrl, onSave }: PdfEditorProps) {
  // URL params (for iframe integration)
  const fileIdParam = getUrlParam('fileId');
  const gasUrlParam = getUrlParam('gasUrl');
  const [driveFileName, setDriveFileName] = useState<string | null>(null);
  const [isSavingToDrive, setIsSavingToDrive] = useState(false);
  // State
  const [mode, setMode] = useState<'add' | 'select'>('add');
  const [inputText, setInputText] = useState('');
  const [fontSize, setFontSize] = useState(12);
  const [fontFamily, setFontFamily] = useState<'mincho' | 'gothic'>('mincho');
  const [annotations, setAnnotations] = useState<TextAnnotation[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [totalPages, setTotalPages] = useState(0);
  const [pdfLoaded, setPdfLoaded] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [history, setHistory] = useState<TextAnnotation[][]>([]);
  const [zoom, setZoom] = useState(1.0);

  // Refs
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const pdfDocRef = useRef<PDFDocumentProxy | null>(null);
  const pdfBytesRef = useRef<Uint8Array | null>(null);
  const fontsRef = useRef<{ mincho: ArrayBuffer | null; gothic: ArrayBuffer | null }>({ mincho: null, gothic: null });
  const dragRef = useRef<{ isDragging: boolean; startX: number; startY: number; annId: string | null }>({
    isDragging: false, startX: 0, startY: 0, annId: null
  });
  const panRef = useRef<{ isPanning: boolean; startX: number; startY: number; scrollLeft: number; scrollTop: number }>({
    isPanning: false, startX: 0, startY: 0, scrollLeft: 0, scrollTop: 0
  });
  const initializedRef = useRef(false);
  const renderTaskRef = useRef<any>(null);

  // フォント読み込み
  useEffect(() => {
    const loadFonts = async () => {
      try {
        const [minchoRes, gothicRes] = await Promise.all([
          fetch('/fonts/NotoSerifJP-Subset.otf'),
          fetch('/fonts/NotoSansJP-Subset.otf')
        ]);
        fontsRef.current.mincho = await minchoRes.arrayBuffer();
        fontsRef.current.gothic = await gothicRes.arrayBuffer();
      } catch (e) {
        console.error('Font load error:', e);
      }
    };
    loadFonts();
  }, []);

  // キャッシュから復元
  useEffect(() => {
    let cancelled = false;

    const restoreFromCache = async () => {
      // 既に初期化済みならスキップ
      if (initializedRef.current) return;

      try {
        const cachedPdf = localStorage.getItem('pdfEditor_pdf');
        const cachedAnnotations = localStorage.getItem('pdfEditor_annotations');

        console.log('Cache check:', {
          hasPdf: !!cachedPdf,
          pdfSize: cachedPdf?.length,
          hasAnnotations: !!cachedAnnotations
        });

        if (cachedPdf && cachedPdf.length > 0) {
          const bytes = Uint8Array.from(atob(cachedPdf), c => c.charCodeAt(0));
          console.log('Restored PDF bytes:', bytes.length);

          if (cancelled) return;

          pdfBytesRef.current = bytes;
          const pdf = await getDocument({ data: bytes.slice() }).promise;

          if (cancelled) return;

          pdfDocRef.current = pdf;
          setTotalPages(pdf.numPages);
          setCurrentPage(1);
          setPdfLoaded(true);
          setStatus('キャッシュから復元しました');
        }

        if (cachedAnnotations && !cancelled) {
          const parsed = JSON.parse(cachedAnnotations);
          console.log('Restored annotations:', parsed.length);
          setAnnotations(parsed);
        }

        initializedRef.current = true;
      } catch (e) {
        console.error('Cache restore error:', e);
        localStorage.removeItem('pdfEditor_pdf');
        localStorage.removeItem('pdfEditor_annotations');
        initializedRef.current = true;
      }
    };
    restoreFromCache();

    return () => { cancelled = true; };
  }, []);

  // 注釈変更時にキャッシュ保存（初期化後のみ）
  useEffect(() => {
    if (!initializedRef.current) return;
    localStorage.setItem('pdfEditor_annotations', JSON.stringify(annotations));
  }, [annotations]);

  // PDFレンダリング
  const baseScale = 1.0; // 基本スケール（100%）

  const renderPage = useCallback(async (pageNum: number) => {
    if (!pdfDocRef.current || !canvasRef.current) return;

    // 前のレンダリングをキャンセル
    if (renderTaskRef.current) {
      renderTaskRef.current.cancel();
      renderTaskRef.current = null;
    }

    const page = await pdfDocRef.current.getPage(pageNum);
    const scale = baseScale * zoom;
    const viewport = page.getViewport({ scale });

    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d')!;
    canvas.width = viewport.width;
    canvas.height = viewport.height;

    // オーバーレイも同じサイズに
    if (overlayRef.current) {
      overlayRef.current.width = viewport.width;
      overlayRef.current.height = viewport.height;
    }

    const renderTask = page.render({
      canvasContext: ctx,
      viewport,
      annotationMode: AnnotationMode.DISABLE,
      canvas,
    });
    renderTaskRef.current = renderTask;

    try {
      await renderTask.promise;
    } catch (e: any) {
      if (e?.name !== 'RenderingCancelledException') {
        console.error('Render error:', e);
      }
    }
  }, [zoom]);

  // オーバーレイ描画（注釈表示）
  const renderOverlay = useCallback(() => {
    if (!overlayRef.current) return;
    const ctx = overlayRef.current.getContext('2d')!;
    ctx.clearRect(0, 0, overlayRef.current.width, overlayRef.current.height);

    const displayScale = baseScale * zoom;
    const pageAnnotations = annotations.filter(a => a.page === currentPage);
    for (const ann of pageAnnotations) {
      ctx.font = `${ann.fontSize * displayScale}px ${ann.fontFamily === 'mincho' ? 'serif' : 'sans-serif'}`;
      ctx.fillStyle = ann.color;
      ctx.fillText(ann.text, ann.x * zoom, ann.y * zoom);

      // 選択中は枠を表示
      if (ann.id === selectedId) {
        const metrics = ctx.measureText(ann.text);
        ctx.strokeStyle = '#0066ff';
        ctx.lineWidth = 2;
        ctx.strokeRect(ann.x * zoom - 2, ann.y * zoom - ann.fontSize * displayScale, metrics.width + 4, ann.fontSize * displayScale * 1.2);
      }
    }
  }, [annotations, currentPage, selectedId, zoom]);

  useEffect(() => {
    renderOverlay();
  }, [renderOverlay]);

  // ズーム変更時に再レンダリング
  useEffect(() => {
    if (pdfLoaded) {
      renderPage(currentPage).then(renderOverlay);
    }
  }, [zoom, pdfLoaded, currentPage, renderPage, renderOverlay]);

  // PDF読み込み
  const loadPdf = async (data: ArrayBuffer) => {
    // データをコピーして保存（getDocumentがArrayBufferを消費するため）
    const bytes = new Uint8Array(data);
    pdfBytesRef.current = bytes.slice();
    const pdf = await getDocument({ data: bytes }).promise;
    pdfDocRef.current = pdf;
    setTotalPages(pdf.numPages);
    setCurrentPage(1);
    setPdfLoaded(true);
    setAnnotations([]);
    setHistory([]);
    await renderPage(1);

    // localStorageにキャッシュ（5MB以下の場合）
    const cachedBytes = pdfBytesRef.current;
    if (cachedBytes && cachedBytes.length < 5 * 1024 * 1024) {
      try {
        // チャンクに分けてbase64変換（スタックオーバーフロー回避）
        let binary = '';
        const chunkSize = 8192;
        for (let i = 0; i < cachedBytes.length; i += chunkSize) {
          binary += String.fromCharCode(...cachedBytes.slice(i, i + chunkSize));
        }
        const base64 = btoa(binary);
        localStorage.setItem('pdfEditor_pdf', base64);
        localStorage.removeItem('pdfEditor_annotations');
        console.log('PDF cached:', base64.length, 'chars, bytes:', cachedBytes.length);
        setStatus('PDF読み込み完了（キャッシュ済）');
      } catch (e) {
        console.error('PDF cache failed:', e);
        setStatus('PDF読み込み完了');
      }
    } else {
      console.log('PDF too large to cache:', cachedBytes?.length);
      setStatus('PDF読み込み完了（大きすぎてキャッシュ不可）');
    }

    // 初期化フラグをセット（新規読み込み時）
    initializedRef.current = true;
  };

  // URL からPDF読み込み
  useEffect(() => {
    if (pdfUrl) {
      fetch(pdfUrl)
        .then(res => res.arrayBuffer())
        .then(loadPdf)
        .catch(e => setStatus(`読み込みエラー: ${e.message}`));
    }
  }, [pdfUrl]);

  // Google DriveからPDF読み込み (URL params: ?fileId=xxx&gasUrl=xxx)
  useEffect(() => {
    if (fileIdParam && gasUrlParam) {
      setStatus('Google Driveから読み込み中...');
      const fetchUrl = `${gasUrlParam}?action=fetchPdf&fileId=${encodeURIComponent(fileIdParam)}`;
      fetch(fetchUrl)
        .then(res => res.json())
        .then(async (result) => {
          if (result.error) throw new Error(result.error);
          if (result.base64) {
            setDriveFileName(result.fileName || 'document.pdf');
            const binary = atob(result.base64);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) {
              bytes[i] = binary.charCodeAt(i);
            }
            await loadPdf(bytes.buffer);
            setStatus(`読み込み完了: ${result.fileName}`);
          }
        })
        .catch(e => setStatus(`読み込みエラー: ${e.message}`));
    }
  }, [fileIdParam, gasUrlParam]);

  // ファイル選択
  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const data = await file.arrayBuffer();
      await loadPdf(data);
    }
  };

  // 履歴保存
  const saveHistory = () => {
    setHistory(prev => [...prev, [...annotations]]);
  };

  // 取り消し
  const handleUndo = () => {
    if (history.length > 0) {
      const prev = history[history.length - 1];
      setAnnotations(prev);
      setHistory(h => h.slice(0, -1));
      setStatus('取り消しました');
    }
  };

  // キャンバスクリック
  const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!pdfLoaded || panRef.current.isPanning) return;

    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const clickY = e.clientY - rect.top;
    // ズームを考慮した実座標
    const x = clickX / zoom;
    const y = clickY / zoom;

    if (mode === 'add' && inputText) {
      saveHistory();
      const newAnn: TextAnnotation = {
        id: `ann_${Date.now()}`,
        x, y,
        text: inputText,
        fontSize,
        fontFamily,
        color: '#000000',
        page: currentPage
      };
      setAnnotations(prev => [...prev, newAnn]);
      setStatus('テキスト追加');
    } else if (mode === 'select') {
      // 注釈の当たり判定
      const ctx = overlayRef.current?.getContext('2d');
      if (!ctx) return;

      const displayScale = baseScale * zoom;
      const pageAnns = annotations.filter(a => a.page === currentPage);
      let found: TextAnnotation | null = null;

      for (const ann of pageAnns) {
        ctx.font = `${ann.fontSize * displayScale}px ${ann.fontFamily === 'mincho' ? 'serif' : 'sans-serif'}`;
        const metrics = ctx.measureText(ann.text);
        const annX = ann.x * zoom;
        const annY = ann.y * zoom;
        if (clickX >= annX - 2 && clickX <= annX + metrics.width + 2 &&
            clickY >= annY - ann.fontSize * displayScale && clickY <= annY + 4) {
          found = ann;
          break;
        }
      }

      if (found) {
        setSelectedId(found.id);
        setInputText(found.text);
        setFontSize(found.fontSize);
        setFontFamily(found.fontFamily);
      } else {
        setSelectedId(null);
      }
    }
  };

  // ドラッグ開始
  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const clickY = e.clientY - rect.top;

    // 中ボタンまたはスペース押しながらでパン開始
    if (e.button === 1) {
      e.preventDefault();
      panRef.current = {
        isPanning: true,
        startX: e.clientX,
        startY: e.clientY,
        scrollLeft: containerRef.current?.scrollLeft || 0,
        scrollTop: containerRef.current?.scrollTop || 0
      };
      return;
    }

    if (mode !== 'select' || !selectedId) return;

    // 選択中の注釈上でマウスダウンしたらドラッグ開始
    const ctx = overlayRef.current?.getContext('2d');
    if (!ctx) return;

    const displayScale = baseScale * zoom;
    const ann = annotations.find(a => a.id === selectedId && a.page === currentPage);
    if (ann) {
      ctx.font = `${ann.fontSize * displayScale}px ${ann.fontFamily === 'mincho' ? 'serif' : 'sans-serif'}`;
      const metrics = ctx.measureText(ann.text);
      const annX = ann.x * zoom;
      const annY = ann.y * zoom;
      if (clickX >= annX - 2 && clickX <= annX + metrics.width + 2 &&
          clickY >= annY - ann.fontSize * displayScale && clickY <= annY + 4) {
        saveHistory();
        dragRef.current = { isDragging: true, startX: clickX, startY: clickY, annId: selectedId };
      }
    }
  };

  // ドラッグ中・パン中
  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    // パン処理
    if (panRef.current.isPanning && containerRef.current) {
      const dx = e.clientX - panRef.current.startX;
      const dy = e.clientY - panRef.current.startY;
      containerRef.current.scrollLeft = panRef.current.scrollLeft - dx;
      containerRef.current.scrollTop = panRef.current.scrollTop - dy;
      return;
    }

    // ドラッグ処理
    if (!dragRef.current.isDragging || !dragRef.current.annId) return;

    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const dx = (x - dragRef.current.startX) / zoom;
    const dy = (y - dragRef.current.startY) / zoom;

    setAnnotations(prev => prev.map(a =>
      a.id === dragRef.current.annId ? { ...a, x: a.x + dx, y: a.y + dy } : a
    ));
    dragRef.current.startX = x;
    dragRef.current.startY = y;
  };

  const handleMouseUp = () => {
    if (dragRef.current.isDragging) {
      dragRef.current.isDragging = false;
      dragRef.current.annId = null;
    }
    if (panRef.current.isPanning) {
      panRef.current.isPanning = false;
    }
  };

  // ホイールでズーム
  const handleWheel = (e: React.WheelEvent) => {
    if (e.ctrlKey) {
      e.preventDefault();
      const delta = e.deltaY > 0 ? -0.1 : 0.1;
      setZoom(z => Math.max(0.25, Math.min(4, z + delta)));
    }
  };

  // 削除
  const handleDelete = () => {
    if (selectedId) {
      saveHistory();
      setAnnotations(prev => prev.filter(a => a.id !== selectedId));
      setSelectedId(null);
      setStatus('削除しました');
    }
  };

  // 選択中の注釈を更新
  useEffect(() => {
    if (selectedId && mode === 'select') {
      setAnnotations(prev => prev.map(a =>
        a.id === selectedId ? { ...a, text: inputText, fontSize, fontFamily } : a
      ));
    }
  }, [inputText, fontSize, fontFamily, selectedId, mode]);

  // PDF保存
  const handleSave = async () => {
    if (!pdfBytesRef.current) return;

    try {
      const pdfDoc = await PDFDocument.load(pdfBytesRef.current);
      pdfDoc.registerFontkit(fontkit);

      // フォント読み込み確認
      if (!fontsRef.current.mincho || !fontsRef.current.gothic) {
        setStatus('フォント読み込み中...');
        return;
      }

      const minchoFont = await pdfDoc.embedFont(fontsRef.current.mincho);
      const gothicFont = await pdfDoc.embedFont(fontsRef.current.gothic);

      const pages = pdfDoc.getPages();
      console.log('Annotations to save:', annotations);

      for (const ann of annotations) {
        const page = pages[ann.page - 1];
        if (!page) continue;

        const font = ann.fontFamily === 'mincho' ? minchoFont : gothicFont;
        const { width, height } = page.getSize();
        const rotation = page.getRotation().angle;

        console.log(`Page ${ann.page}: size=${width}x${height}, rotation=${rotation}`);

        // ページ回転を考慮した座標変換（JS版と同じロジック）
        let pdfX: number, pdfY: number;
        if (rotation === 90) {
          pdfX = ann.y;
          pdfY = ann.x;
        } else if (rotation === 180) {
          pdfX = width - ann.x;
          pdfY = ann.y;
        } else if (rotation === 270) {
          pdfX = height - ann.y;
          pdfY = width - ann.x;
        } else {
          // 0度または回転なし
          pdfX = ann.x;
          pdfY = height - ann.y;
        }

        console.log(`Drawing "${ann.text}" at PDF coords: (${pdfX}, ${pdfY}), rotation: ${rotation}`);

        page.drawText(ann.text, {
          x: pdfX,
          y: pdfY,
          size: ann.fontSize,
          font,
          color: rgb(0, 0, 0),
          rotate: degrees(rotation),
        });
      }

      const savedBytes = await pdfDoc.save();

      // ダウンロード
      const blob = new Blob([new Uint8Array(savedBytes)], { type: 'application/pdf' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'edited.pdf';
      a.click();
      URL.revokeObjectURL(url);

      if (onSave) {
        onSave(savedBytes);
      }
      setStatus('保存しました');
    } catch (e) {
      console.error('Save error:', e);
      setStatus(`保存エラー: ${e}`);
    }
  };

  // Google Driveに保存
  const handleSaveToDrive = async () => {
    if (!pdfBytesRef.current || !gasUrlParam || !fileIdParam) return;
    setIsSavingToDrive(true);
    setStatus('Driveに保存中...');

    try {
      const pdfDoc = await PDFDocument.load(pdfBytesRef.current);
      pdfDoc.registerFontkit(fontkit);

      if (!fontsRef.current.mincho || !fontsRef.current.gothic) {
        setStatus('フォント読み込み中...');
        setIsSavingToDrive(false);
        return;
      }

      const minchoFont = await pdfDoc.embedFont(fontsRef.current.mincho);
      const gothicFont = await pdfDoc.embedFont(fontsRef.current.gothic);
      const pages = pdfDoc.getPages();

      for (const ann of annotations) {
        const page = pages[ann.page - 1];
        if (!page) continue;
        const font = ann.fontFamily === 'mincho' ? minchoFont : gothicFont;
        const { width, height } = page.getSize();
        const rotation = page.getRotation().angle;

        let pdfX: number, pdfY: number;
        if (rotation === 90) { pdfX = ann.y; pdfY = ann.x; }
        else if (rotation === 180) { pdfX = width - ann.x; pdfY = ann.y; }
        else if (rotation === 270) { pdfX = height - ann.y; pdfY = width - ann.x; }
        else { pdfX = ann.x; pdfY = height - ann.y; }

        page.drawText(ann.text, { x: pdfX, y: pdfY, size: ann.fontSize, font, color: rgb(0, 0, 0), rotate: degrees(rotation) });
      }

      const savedBytes = await pdfDoc.save();
      let binary = '';
      const bytes = new Uint8Array(savedBytes);
      const chunkSize = 8192;
      for (let i = 0; i < bytes.length; i += chunkSize) {
        binary += String.fromCharCode(...bytes.slice(i, i + chunkSize));
      }
      const base64 = btoa(binary);

      const response = await fetch(gasUrlParam, {
        method: 'POST',
        body: JSON.stringify({
          action: 'uploadPdf',
          base64,
          originalFileId: fileIdParam,
          newFileName: driveFileName,
          overwrite: true
        })
      });
      const result = await response.json();
      if (result.error) throw new Error(result.error);
      setStatus(`Driveに保存しました: ${result.fileName}`);
    } catch (e) {
      console.error('Drive save error:', e);
      setStatus(`Drive保存エラー: ${e}`);
    } finally {
      setIsSavingToDrive(false);
    }
  };

  // ページ移動
  const goToPage = (page: number) => {
    if (page >= 1 && page <= totalPages) {
      setCurrentPage(page);
      renderPage(page);
      setSelectedId(null);
    }
  };

  // キーボードショートカット
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.ctrlKey && e.key === 'z') {
      e.preventDefault();
      handleUndo();
    }
    if (e.key === 'Delete' && selectedId) {
      handleDelete();
    }
  };

  return (
    <div className="pdf-editor" tabIndex={0} onKeyDown={handleKeyDown}>
      <div className="toolbar">
        <label className="file-button">
          開く
          <input type="file" accept=".pdf" onChange={handleFileChange} />
        </label>

        <button onClick={handleSave} disabled={!pdfLoaded} className="save-btn">💾保存</button>
        {fileIdParam && gasUrlParam && (
          <button onClick={handleSaveToDrive} disabled={!pdfLoaded || isSavingToDrive} className="save-btn" style={{background: '#34a853', borderColor: '#34a853'}}>
            ☁️Drive保存
          </button>
        )}

        <span className="separator">|</span>

        <div className="mode-switch">
          <span className={mode === 'select' ? 'active' : ''}>選択</span>
          <label className="switch">
            <input
              type="checkbox"
              checked={mode === 'add'}
              onChange={() => setMode(mode === 'add' ? 'select' : 'add')}
            />
            <span className="slider"></span>
          </label>
          <span className={mode === 'add' ? 'active' : ''}>追加</span>
        </div>

        <input
          type="text"
          className="text-input"
          placeholder="テキスト"
          value={inputText}
          onChange={e => setInputText(e.target.value)}
          disabled={!pdfLoaded}
        />

        <select
          value={fontSize}
          onChange={e => setFontSize(Number(e.target.value))}
          disabled={!pdfLoaded}
        >
          {[10, 12, 14, 16, 18, 20, 24].map(s => (
            <option key={s} value={s}>{s}pt</option>
          ))}
        </select>

        <select
          value={fontFamily}
          onChange={e => setFontFamily(e.target.value as 'mincho' | 'gothic')}
          disabled={!pdfLoaded}
        >
          <option value="mincho">明朝</option>
          <option value="gothic">ゴシック</option>
        </select>

        <button onClick={handleUndo} disabled={history.length === 0}>↩</button>
        <button onClick={handleDelete} disabled={!selectedId} className="delete-btn">✕</button>
      </div>

      {status && <div className="status">{status}</div>}
      {driveFileName && <div className="status" style={{background: '#e8f0fe', color: '#1967d2'}}>📁 {driveFileName}</div>}

      <div className="page-nav">
        <button onClick={() => goToPage(currentPage - 1)} disabled={currentPage <= 1}>◀</button>
        <span>{currentPage} / {totalPages}</span>
        <button onClick={() => goToPage(currentPage + 1)} disabled={currentPage >= totalPages}>▶</button>
        <span className="zoom-indicator">|</span>
        <button onClick={() => setZoom(z => Math.max(0.25, z - 0.25))}>−</button>
        <span className="zoom-value">{Math.round(zoom * 100)}%</span>
        <button onClick={() => setZoom(z => Math.min(4, z + 0.25))}>+</button>
        <button onClick={() => setZoom(1)}>100%</button>
      </div>

      <div
        className="canvas-container"
        ref={containerRef}
        onWheel={handleWheel}
      >
        {!pdfLoaded && (
          <div className="file-upload-hint">
            「開く」ボタンでPDFを選択
          </div>
        )}
        <div className="canvas-wrapper">
          <canvas ref={canvasRef} className="pdf-canvas" />
          <canvas
            ref={overlayRef}
            className="overlay-canvas"
            onClick={handleCanvasClick}
            onMouseDown={handleMouseDown}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseUp}
          />
        </div>
      </div>
    </div>
  );
}
