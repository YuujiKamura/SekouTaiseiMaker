import { useState, useRef, useEffect, useCallback } from 'react';
import { PDFDocument, rgb, degrees } from 'pdf-lib';
import fontkit from '@pdf-lib/fontkit';
import { getDocument, GlobalWorkerOptions, AnnotationMode } from 'pdfjs-dist';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import { getCachedPdf, setCachedPdf, isCacheValid, invalidateCache } from '../services/pdfCache';
import { createFontSubset } from '../utils/fontSubset';
import { safeBase64ToArrayBuffer } from '../utils/base64';
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

interface RectAnnotation {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
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
  // State
  const [mode, setMode] = useState<'text' | 'rect' | 'select'>('text');
  const [inputText, setInputText] = useState('');
  const [fontSize, setFontSize] = useState(12);
  const [fontFamily, setFontFamily] = useState<'mincho' | 'gothic'>('mincho');
  const [rectColor, setRectColor] = useState('#808080'); // グレーがデフォルト
  const [annotations, setAnnotations] = useState<TextAnnotation[]>([]);
  const [rectangles, setRectangles] = useState<RectAnnotation[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedType, setSelectedType] = useState<'text' | 'rect' | null>(null);
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
  const rectDrawRef = useRef<{ isDrawing: boolean; startX: number; startY: number; currentX: number; currentY: number }>({
    isDrawing: false, startX: 0, startY: 0, currentX: 0, currentY: 0
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
          fetch('./fonts/NotoSerifJP-Subset.otf'),
          fetch('./fonts/NotoSansJP-Subset.otf')
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
        const cachedRectangles = localStorage.getItem('pdfEditor_rectangles');

        console.log('Cache check:', {
          hasPdf: !!cachedPdf,
          pdfSize: cachedPdf?.length,
          hasAnnotations: !!cachedAnnotations,
          hasRectangles: !!cachedRectangles
        });

        if (cachedPdf && cachedPdf.length > 0) {
          const bytes = new Uint8Array(safeBase64ToArrayBuffer(cachedPdf));
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

        if (cachedRectangles && !cancelled) {
          const parsed = JSON.parse(cachedRectangles);
          console.log('Restored rectangles:', parsed.length);
          setRectangles(parsed);
        }

        initializedRef.current = true;
      } catch (e) {
        console.error('Cache restore error:', e);
        localStorage.removeItem('pdfEditor_pdf');
        localStorage.removeItem('pdfEditor_annotations');
        localStorage.removeItem('pdfEditor_rectangles');
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

  // 矩形変更時にキャッシュ保存（初期化後のみ）
  useEffect(() => {
    if (!initializedRef.current) return;
    localStorage.setItem('pdfEditor_rectangles', JSON.stringify(rectangles));
  }, [rectangles]);

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

  // オーバーレイ描画（注釈・矩形表示）
  const renderOverlay = useCallback(() => {
    if (!overlayRef.current) return;
    const ctx = overlayRef.current.getContext('2d')!;
    ctx.clearRect(0, 0, overlayRef.current.width, overlayRef.current.height);

    const displayScale = baseScale * zoom;

    // 矩形を描画（テキストより先に描画してテキストが上に来るように）
    const pageRectangles = rectangles.filter(r => r.page === currentPage);
    for (const rect of pageRectangles) {
      ctx.fillStyle = rect.color;
      ctx.fillRect(rect.x * zoom, rect.y * zoom, rect.width * zoom, rect.height * zoom);

      // 選択中は枠を表示
      if (rect.id === selectedId && selectedType === 'rect') {
        ctx.strokeStyle = '#0066ff';
        ctx.lineWidth = 2;
        ctx.strokeRect(rect.x * zoom - 1, rect.y * zoom - 1, rect.width * zoom + 2, rect.height * zoom + 2);
      }
    }

    // 矩形描画中のプレビュー
    if (rectDrawRef.current.isDrawing) {
      const { startX, startY, currentX, currentY } = rectDrawRef.current;
      const x = Math.min(startX, currentX);
      const y = Math.min(startY, currentY);
      const w = Math.abs(currentX - startX);
      const h = Math.abs(currentY - startY);
      ctx.fillStyle = rectColor + '80'; // 半透明
      ctx.fillRect(x, y, w, h);
      ctx.strokeStyle = rectColor;
      ctx.lineWidth = 1;
      ctx.strokeRect(x, y, w, h);
    }

    // テキスト注釈を描画
    const pageAnnotations = annotations.filter(a => a.page === currentPage);
    for (const ann of pageAnnotations) {
      ctx.font = `${ann.fontSize * displayScale}px ${ann.fontFamily === 'mincho' ? 'serif' : 'sans-serif'}`;
      ctx.fillStyle = ann.color;
      ctx.fillText(ann.text, ann.x * zoom, ann.y * zoom);

      // 選択中は枠を表示
      if (ann.id === selectedId && selectedType === 'text') {
        const metrics = ctx.measureText(ann.text);
        ctx.strokeStyle = '#0066ff';
        ctx.lineWidth = 2;
        ctx.strokeRect(ann.x * zoom - 2, ann.y * zoom - ann.fontSize * displayScale, metrics.width + 4, ann.fontSize * displayScale * 1.2);
      }
    }
  }, [annotations, rectangles, currentPage, selectedId, selectedType, zoom, rectColor]);

  // ズーム・ページ変更時に再レンダリング
  useEffect(() => {
    if (pdfLoaded) {
      renderPage(currentPage).then(() => {
        // renderPage完了後に必ずオーバーレイを再描画
        setTimeout(() => renderOverlay(), 0);
      });
    }
  }, [zoom, pdfLoaded, currentPage, renderPage, renderOverlay]);

  // ピンチズーム（二本指ズーム）対応
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    let initialDistance = 0;
    let initialZoom = zoom;

    const getDistance = (touches: TouchList): number => {
      if (touches.length < 2) return 0;
      const dx = touches[0].clientX - touches[1].clientX;
      const dy = touches[0].clientY - touches[1].clientY;
      return Math.sqrt(dx * dx + dy * dy);
    };

    const handleTouchStart = (e: TouchEvent) => {
      if (e.touches.length === 2) {
        e.preventDefault();
        initialDistance = getDistance(e.touches);
        initialZoom = zoom;
      }
    };

    const handleTouchMove = (e: TouchEvent) => {
      if (e.touches.length === 2 && initialDistance > 0) {
        e.preventDefault();
        const currentDistance = getDistance(e.touches);
        const scale = currentDistance / initialDistance;
        const newZoom = Math.min(Math.max(initialZoom * scale, 0.25), 4.0);
        setZoom(newZoom);
      }
    };

    const handleTouchEnd = () => {
      initialDistance = 0;
    };

    container.addEventListener('touchstart', handleTouchStart, { passive: false });
    container.addEventListener('touchmove', handleTouchMove, { passive: false });
    container.addEventListener('touchend', handleTouchEnd);

    return () => {
      container.removeEventListener('touchstart', handleTouchStart);
      container.removeEventListener('touchmove', handleTouchMove);
      container.removeEventListener('touchend', handleTouchEnd);
    };
  }, [zoom]);

  // annotations/rectangles変更時にオーバーレイを再描画（PDFレンダリング不要）
  useEffect(() => {
    if (pdfLoaded && overlayRef.current && overlayRef.current.width > 0) {
      renderOverlay();
    }
  }, [annotations, rectangles, selectedId, selectedType, pdfLoaded, renderOverlay]);

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
    setRectangles([]);
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
        localStorage.removeItem('pdfEditor_rectangles');
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
      const loadFromDrive = async () => {
        // まずGASからファイル情報（modifiedTime）を取得
        setStatus('ファイル情報を確認中...');
        const infoUrl = `${gasUrlParam}?action=getFileInfo&fileId=${encodeURIComponent(fileIdParam)}`;
        let modifiedTime: string | undefined;
        let fileName = 'document.pdf';
        
        try {
          const infoRes = await fetch(infoUrl, { cache: 'no-store' });
          const info = await infoRes.json();
          if (!info.error) {
            modifiedTime = info.modifiedTime;
            fileName = info.fileName || fileName;
            setDriveFileName(fileName);
          }
        } catch {
          // ファイル情報取得失敗は無視（キャッシュ検証をスキップ）
        }

        // キャッシュをチェック（modifiedTimeで検証）
        if (modifiedTime) {
          const cacheValid = await isCacheValid(fileIdParam, modifiedTime);
          if (cacheValid) {
            const cached = await getCachedPdf(fileIdParam);
            if (cached) {
              setStatus('キャッシュから読み込み中...');
              await loadPdf(cached);
              setStatus('読み込み完了 (キャッシュ)');
              return;
            }
          } else {
            // キャッシュが古い場合は削除
            await invalidateCache(fileIdParam);
            console.log('Cache invalidated: file was modified');
          }
        }

        // GASからPDFを取得
        setStatus('Google Driveから読み込み中...');
        const fetchUrl = `${gasUrlParam}?action=fetchPdf&fileId=${encodeURIComponent(fileIdParam)}`;
        const res = await fetch(fetchUrl, { cache: 'no-store' });
        const result = await res.json();
        if (result.error) throw new Error(result.error);
        if (result.base64) {
          setDriveFileName(result.fileName || fileName);
          // Base64をArrayBufferに変換（sanitization付き）
          const pdfBytes = safeBase64ToArrayBuffer(result.base64);
          // キャッシュに保存（modifiedTime付き）
          await setCachedPdf(fileIdParam, pdfBytes, modifiedTime || result.modifiedTime);
          await loadPdf(pdfBytes);
          setStatus(`読み込み完了: ${result.fileName || fileName}`);
        }
      };
      loadFromDrive().catch(e => setStatus(`読み込みエラー: ${e.message}`));
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

    if (mode === 'text' && inputText) {
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
      // まず矩形の当たり判定
      const pageRects = rectangles.filter(r => r.page === currentPage);
      let foundRect: RectAnnotation | null = null;

      for (const rect of pageRects) {
        const rx = rect.x * zoom;
        const ry = rect.y * zoom;
        const rw = rect.width * zoom;
        const rh = rect.height * zoom;
        if (clickX >= rx && clickX <= rx + rw && clickY >= ry && clickY <= ry + rh) {
          foundRect = rect;
          break;
        }
      }

      if (foundRect) {
        setSelectedId(foundRect.id);
        setSelectedType('rect');
        setRectColor(foundRect.color);
        return;
      }

      // テキスト注釈の当たり判定
      const ctx = overlayRef.current?.getContext('2d');
      if (!ctx) return;

      const displayScale = baseScale * zoom;
      const pageAnns = annotations.filter(a => a.page === currentPage);
      let foundText: TextAnnotation | null = null;

      for (const ann of pageAnns) {
        ctx.font = `${ann.fontSize * displayScale}px ${ann.fontFamily === 'mincho' ? 'serif' : 'sans-serif'}`;
        const metrics = ctx.measureText(ann.text);
        const annX = ann.x * zoom;
        const annY = ann.y * zoom;
        if (clickX >= annX - 2 && clickX <= annX + metrics.width + 2 &&
            clickY >= annY - ann.fontSize * displayScale && clickY <= annY + 4) {
          foundText = ann;
          break;
        }
      }

      if (foundText) {
        setSelectedId(foundText.id);
        setSelectedType('text');
        setInputText(foundText.text);
        setFontSize(foundText.fontSize);
        setFontFamily(foundText.fontFamily);
      } else {
        setSelectedId(null);
        setSelectedType(null);
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

    // 矩形モードでドラッグ開始
    if (mode === 'rect') {
      rectDrawRef.current = {
        isDrawing: true,
        startX: clickX,
        startY: clickY,
        currentX: clickX,
        currentY: clickY
      };
      return;
    }

    if (mode !== 'select' || !selectedId) return;

    // 選択中のテキスト注釈上でマウスダウンしたらドラッグ開始
    if (selectedType === 'text') {
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
    } else if (selectedType === 'rect') {
      // 選択中の矩形上でマウスダウンしたらドラッグ開始
      const rectItem = rectangles.find(r => r.id === selectedId && r.page === currentPage);
      if (rectItem) {
        const rx = rectItem.x * zoom;
        const ry = rectItem.y * zoom;
        const rw = rectItem.width * zoom;
        const rh = rectItem.height * zoom;
        if (clickX >= rx && clickX <= rx + rw && clickY >= ry && clickY <= ry + rh) {
          saveHistory();
          dragRef.current = { isDragging: true, startX: clickX, startY: clickY, annId: selectedId };
        }
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

    // 矩形描画中
    if (rectDrawRef.current.isDrawing) {
      const rect = e.currentTarget.getBoundingClientRect();
      rectDrawRef.current.currentX = e.clientX - rect.left;
      rectDrawRef.current.currentY = e.clientY - rect.top;
      renderOverlay();
      return;
    }

    // ドラッグ処理（テキストまたは矩形の移動）
    if (!dragRef.current.isDragging || !dragRef.current.annId) return;

    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const dx = (x - dragRef.current.startX) / zoom;
    const dy = (y - dragRef.current.startY) / zoom;

    if (selectedType === 'text') {
      setAnnotations(prev => prev.map(a =>
        a.id === dragRef.current.annId ? { ...a, x: a.x + dx, y: a.y + dy } : a
      ));
    } else if (selectedType === 'rect') {
      setRectangles(prev => prev.map(r =>
        r.id === dragRef.current.annId ? { ...r, x: r.x + dx, y: r.y + dy } : r
      ));
    }
    dragRef.current.startX = x;
    dragRef.current.startY = y;
  };

  const handleMouseUp = () => {
    // 矩形描画完了
    if (rectDrawRef.current.isDrawing) {
      const { startX, startY, currentX, currentY } = rectDrawRef.current;
      const x = Math.min(startX, currentX) / zoom;
      const y = Math.min(startY, currentY) / zoom;
      const width = Math.abs(currentX - startX) / zoom;
      const height = Math.abs(currentY - startY) / zoom;

      // 最小サイズのチェック
      if (width >= 5 && height >= 5) {
        saveHistory();
        const newRect: RectAnnotation = {
          id: `rect_${Date.now()}`,
          x, y, width, height,
          color: rectColor,
          page: currentPage
        };
        setRectangles(prev => [...prev, newRect]);
        setStatus('矩形追加');
      }

      rectDrawRef.current.isDrawing = false;
      renderOverlay();
    }

    if (dragRef.current.isDragging) {
      dragRef.current.isDragging = false;
      dragRef.current.annId = null;
    }
    if (panRef.current.isPanning) {
      panRef.current.isPanning = false;
    }
  };

  // ホイールでパン（Ctrl+wheelはブラウザズームと競合するので無効化）
  const handleWheel = (_e: React.WheelEvent) => {
    // 通常のスクロール動作を許可（何もしない）
  };

  // 削除
  const handleDelete = () => {
    if (selectedId) {
      saveHistory();
      if (selectedType === 'text') {
        setAnnotations(prev => prev.filter(a => a.id !== selectedId));
      } else if (selectedType === 'rect') {
        setRectangles(prev => prev.filter(r => r.id !== selectedId));
      }
      setSelectedId(null);
      setSelectedType(null);
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

      const pages = pdfDoc.getPages();
      console.log('Annotations to save:', annotations);
      console.log('Rectangles to save:', rectangles);

      // 使用する文字だけを含むサブセットフォントを動的に作成
      const minchoTexts = annotations.filter(a => a.fontFamily === 'mincho').map(a => a.text);
      const gothicTexts = annotations.filter(a => a.fontFamily === 'gothic').map(a => a.text);
      const minchoChars = [...new Set(minchoTexts.join(''))].join('');
      const gothicChars = [...new Set(gothicTexts.join(''))].join('');

      let minchoFont: Awaited<ReturnType<typeof pdfDoc.embedFont>> | null = null;
      let gothicFont: Awaited<ReturnType<typeof pdfDoc.embedFont>> | null = null;

      if (minchoChars || gothicChars) {
        if (!fontsRef.current.mincho || !fontsRef.current.gothic) {
          setStatus('フォント読み込み中...');
          return;
        }

        try {
          // 動的サブセット作成（使用文字のみ）
          if (minchoChars) {
            console.log(`Creating mincho subset: "${minchoChars}" (${minchoChars.length} chars)`);
            const minchoSubset = await createFontSubset(fontsRef.current.mincho, minchoChars);
            console.log(`Mincho subset: ${minchoSubset.byteLength} bytes`);
            minchoFont = await pdfDoc.embedFont(minchoSubset);
          }
          if (gothicChars) {
            console.log(`Creating gothic subset: "${gothicChars}" (${gothicChars.length} chars)`);
            const gothicSubset = await createFontSubset(fontsRef.current.gothic, gothicChars);
            console.log(`Gothic subset: ${gothicSubset.byteLength} bytes`);
            gothicFont = await pdfDoc.embedFont(gothicSubset);
          }
        } catch (subsetError) {
          console.error('Subset error, falling back to full font:', subsetError);
          // フォールバック: フル埋め込み
          if (minchoChars) {
            minchoFont = await pdfDoc.embedFont(fontsRef.current.mincho);
          }
          if (gothicChars) {
            gothicFont = await pdfDoc.embedFont(fontsRef.current.gothic);
          }
        }
      }

      // 矩形を描画（先に描画してテキストが上に来るように）
      for (const rect of rectangles) {
        const page = pages[rect.page - 1];
        if (!page) continue;

        const { width, height } = page.getSize();
        const rotation = page.getRotation().angle;

        // 色を解析 (#808080 -> rgb)
        const hex = rect.color.replace('#', '');
        const r = parseInt(hex.substring(0, 2), 16) / 255;
        const g = parseInt(hex.substring(2, 4), 16) / 255;
        const b = parseInt(hex.substring(4, 6), 16) / 255;

        // ページ回転を考慮した座標変換
        let pdfX: number, pdfY: number, pdfWidth: number, pdfHeight: number;
        if (rotation === 90) {
          pdfX = rect.y;
          pdfY = rect.x;
          pdfWidth = rect.height;
          pdfHeight = rect.width;
        } else if (rotation === 180) {
          pdfX = width - rect.x - rect.width;
          pdfY = rect.y;
          pdfWidth = rect.width;
          pdfHeight = rect.height;
        } else if (rotation === 270) {
          pdfX = height - rect.y - rect.height;
          pdfY = width - rect.x - rect.width;
          pdfWidth = rect.height;
          pdfHeight = rect.width;
        } else {
          // 0度または回転なし
          pdfX = rect.x;
          pdfY = height - rect.y - rect.height;
          pdfWidth = rect.width;
          pdfHeight = rect.height;
        }

        console.log(`Drawing rect at PDF coords: (${pdfX}, ${pdfY}), size: ${pdfWidth}x${pdfHeight}`);

        page.drawRectangle({
          x: pdfX,
          y: pdfY,
          width: pdfWidth,
          height: pdfHeight,
          color: rgb(r, g, b),
        });
      }

      // テキスト注釈を描画
      for (const ann of annotations) {
        const page = pages[ann.page - 1];
        if (!page) continue;

        const font = ann.fontFamily === 'mincho' ? minchoFont : gothicFont;
        if (!font) continue; // フォントがない場合はスキップ
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

      // キャッシュを更新（fileIdがある場合）
      // 現在時刻をmodifiedTimeとして設定
      if (fileIdParam) {
        const nowModifiedTime = new Date().toISOString();
        await setCachedPdf(fileIdParam, savedBytes.slice().buffer, nowModifiedTime);
        console.log('Cache updated for:', fileIdParam, 'with modifiedTime:', nowModifiedTime);
      }

      // 内部状態も更新（再編集に備える）
      pdfBytesRef.current = new Uint8Array(savedBytes);

      // ダウンロード
      const blob = new Blob([new Uint8Array(savedBytes)], { type: 'application/pdf' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = driveFileName || 'edited.pdf';
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
    const target = e.target as HTMLElement;
    const isInputFocused = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT';

    // Ctrl+Z: 元に戻す（入力中でも有効）
    if (e.ctrlKey && e.key === 'z') {
      e.preventDefault();
      handleUndo();
      return;
    }
    // Delete: 選択中のアイテムを削除（入力中は無効）
    if (e.key === 'Delete' && selectedId && !isInputFocused) {
      handleDelete();
      return;
    }
    // 入力フィールドにフォーカス中はZ/Xズームを無効化
    if (isInputFocused) return;

    // Z: 縮小 (zoom out)
    if (e.key === 'z' && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      setZoom(z => Math.max(0.25, z - 0.1));
    }
    // X: 拡大 (zoom in)
    if (e.key === 'x' && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      setZoom(z => Math.min(4, z + 0.1));
    }
  };

  return (
    <div className="pdf-editor" tabIndex={0} onKeyDown={handleKeyDown}>
      <div className="toolbar">
        <button
          className="back-btn"
          onClick={() => {
            // 親ウィンドウに戻るメッセージを送信（iframe統合用）
            if (window.parent !== window) {
              window.parent.postMessage({ type: 'back' }, '*');
            }
            // 常にhistory.back()を試行
            window.history.back();
          }}
          title="ダッシュボードに戻る"
        >
          ← 戻る
        </button>

        <span className="separator">|</span>

        <label className="file-button">
          開く
          <input type="file" accept=".pdf" onChange={handleFileChange} />
        </label>

        <button onClick={handleSave} disabled={!pdfLoaded} className="save-btn">保存</button>

        <span className="separator">|</span>

        <div className="mode-buttons">
          <button
            className={mode === 'select' ? 'active' : ''}
            onClick={() => setMode('select')}
            title="選択モード"
          >選択</button>
          <button
            className={mode === 'text' ? 'active' : ''}
            onClick={() => setMode('text')}
            title="テキスト追加モード"
          >文字</button>
          <button
            className={mode === 'rect' ? 'active' : ''}
            onClick={() => setMode('rect')}
            title="矩形追加モード（マスク）"
          >矩形</button>
        </div>

        {mode === 'text' && (
          <>
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
          </>
        )}

        {mode === 'rect' && (
          <div className="rect-color-picker">
            <label>色:</label>
            <input
              type="color"
              value={rectColor}
              onChange={e => setRectColor(e.target.value)}
              disabled={!pdfLoaded}
            />
            <button
              onClick={() => setRectColor('#808080')}
              className={rectColor === '#808080' ? 'active' : ''}
            >グレー</button>
            <button
              onClick={() => setRectColor('#ffffff')}
              className={rectColor === '#ffffff' ? 'active' : ''}
            >白</button>
            <button
              onClick={() => setRectColor('#000000')}
              className={rectColor === '#000000' ? 'active' : ''}
            >黒</button>
          </div>
        )}

        <button onClick={handleUndo} disabled={history.length === 0}>↩</button>
        <button onClick={handleDelete} disabled={!selectedId} className="delete-btn">✕</button>

        <span className="separator">|</span>

        <button onClick={() => goToPage(currentPage - 1)} disabled={currentPage <= 1}>◀</button>
        <span className="page-info">{currentPage}/{totalPages}</span>
        <button onClick={() => goToPage(currentPage + 1)} disabled={currentPage >= totalPages}>▶</button>

        <span className="separator">|</span>

        <span className="zoom-value">{Math.round(zoom * 100)}%</span>

        {(status || driveFileName) && (
          <>
            <span className="separator">|</span>
            <span className="status-inline">{driveFileName || status}</span>
          </>
        )}
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
