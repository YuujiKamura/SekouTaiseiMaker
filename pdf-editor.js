/**
 * PDF Editor Module
 * PDFにテキストを追加・選択・移動・削除する機能
 */

window.PdfEditor = (function() {
    let pdfDoc = null;
    let pdfBytes = null;
    let currentPage = 1;
    let totalPages = 0;
    let scale = 1.5;
    let textAnnotations = [];
    let canvas = null;
    let ctx = null;
    let overlayCanvas = null;
    let overlayCtx = null;

    // 現在の設定
    let currentFontSize = 12;
    let currentFontFamily = 'mincho';  // 'mincho' or 'gothic'
    let currentColor = '#000000';

    // 選択・移動用の状態
    let editMode = 'add';       // 'add' | 'select'
    let selectedId = null;
    let hoveredId = null;
    let isDragging = false;
    let dragStartX = 0;
    let dragStartY = 0;
    let dragOffsetX = 0;
    let dragOffsetY = 0;
    let annotationIdCounter = 0;

    // PDF識別用
    let currentPdfId = null;
    const STORAGE_PREFIX = 'pdfEditor_annotations_';

    // 日本語フォントキャッシュ
    let fontCache = {
        mincho: null,
        gothic: null
    };

    // 日本語フォントURL (ローカルサブセットフォント - 軽量版)
    const FONT_URLS = {
        gothic: './fonts/NotoSansJP-Subset.otf',
        mincho: './fonts/NotoSerifJP-Subset.otf'
    };

    /**
     * 日本語フォントを取得（キャッシュあり）
     */
    async function getJapaneseFont(family) {
        const key = family === 'mincho' ? 'mincho' : 'gothic';

        if (fontCache[key]) {
            return fontCache[key];
        }

        try {
            const response = await fetch(FONT_URLS[key]);
            if (!response.ok) {
                throw new Error(`Font fetch failed: ${response.status}`);
            }
            const fontBytes = await response.arrayBuffer();
            fontCache[key] = new Uint8Array(fontBytes);
            return fontCache[key];
        } catch (e) {
            console.error('Failed to load Japanese font:', e);
            throw new Error('日本語フォントの読み込みに失敗しました');
        }
    }

    /**
     * PDFのユニークIDを生成（サイズ + 先頭バイトのハッシュ）
     */
    function generatePdfId(bytes) {
        const size = bytes.length;
        // 先頭1024バイトの簡易ハッシュ
        let hash = 0;
        const sampleSize = Math.min(1024, bytes.length);
        for (let i = 0; i < sampleSize; i++) {
            hash = ((hash << 5) - hash) + bytes[i];
            hash = hash & hash; // Convert to 32bit integer
        }
        return `pdf_${size}_${Math.abs(hash)}`;
    }

    /**
     * 注釈をlocalStorageに保存
     */
    function saveAnnotationsToStorage() {
        if (!currentPdfId) return;
        try {
            const data = {
                annotations: textAnnotations,
                annotationIdCounter: annotationIdCounter,
                savedAt: Date.now()
            };
            localStorage.setItem(STORAGE_PREFIX + currentPdfId, JSON.stringify(data));
        } catch (e) {
            console.warn('Failed to save annotations to localStorage:', e);
        }
    }

    /**
     * localStorageから注釈を読み込み
     */
    function loadAnnotationsFromStorage() {
        if (!currentPdfId) return false;
        try {
            const stored = localStorage.getItem(STORAGE_PREFIX + currentPdfId);
            if (stored) {
                const data = JSON.parse(stored);
                textAnnotations = data.annotations || [];
                annotationIdCounter = data.annotationIdCounter || textAnnotations.length;
                return true;
            }
        } catch (e) {
            console.warn('Failed to load annotations from localStorage:', e);
        }
        return false;
    }

    /**
     * 現在のPDFの注釈をクリア（localStorageからも削除）
     */
    function clearAnnotations() {
        textAnnotations = [];
        selectedId = null;
        hoveredId = null;
        annotationIdCounter = 0;
        if (currentPdfId) {
            try {
                localStorage.removeItem(STORAGE_PREFIX + currentPdfId);
            } catch (e) {
                console.warn('Failed to clear annotations from localStorage:', e);
            }
        }
        redrawAnnotations();
    }

    // PDF.js worker設定
    if (typeof pdfjsLib !== 'undefined') {
        pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';
    }

    /**
     * PDFファイルを読み込む
     */
    async function loadPdf(file) {
        return new Promise((resolve, reject) => {
            const reader = new FileReader();
            reader.onload = async function(e) {
                try {
                    pdfBytes = new Uint8Array(e.target.result);
                    // pdf.jsにはコピーを渡す（Workerに転送されると元の配列が使用不能になるため）
                    const loadingTask = pdfjsLib.getDocument({ data: pdfBytes.slice() });
                    pdfDoc = await loadingTask.promise;
                    totalPages = pdfDoc.numPages;
                    currentPage = 1;
                    selectedId = null;
                    hoveredId = null;

                    // PDF IDを生成し、保存済み注釈があれば読み込む
                    currentPdfId = generatePdfId(pdfBytes);
                    if (!loadAnnotationsFromStorage()) {
                        textAnnotations = [];
                        annotationIdCounter = 0;
                    }

                    resolve({ totalPages: totalPages, annotationsRestored: textAnnotations.length > 0 });
                } catch (err) {
                    reject(err);
                }
            };
            reader.onerror = reject;
            reader.readAsArrayBuffer(file);
        });
    }

    /**
     * Base64文字列からPDFを読み込む
     */
    async function loadPdfFromBase64(base64String) {
        try {
            // Base64をバイナリに変換
            const binaryString = atob(base64String);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }

            pdfBytes = bytes;
            // pdf.jsにはコピーを渡す（Workerに転送されると元の配列が使用不能になるため）
            const loadingTask = pdfjsLib.getDocument({ data: pdfBytes.slice() });
            pdfDoc = await loadingTask.promise;
            totalPages = pdfDoc.numPages;
            currentPage = 1;
            selectedId = null;
            hoveredId = null;

            // PDF IDを生成し、保存済み注釈があれば読み込む
            currentPdfId = generatePdfId(pdfBytes);
            if (!loadAnnotationsFromStorage()) {
                textAnnotations = [];
                annotationIdCounter = 0;
            }

            return { totalPages: totalPages, annotationsRestored: textAnnotations.length > 0 };
        } catch (err) {
            throw err;
        }
    }

    /**
     * 指定ページをキャンバスに描画
     */
    async function renderPage(pageNum, canvasId, overlayCanvasId) {
        if (!pdfDoc) return;

        canvas = document.getElementById(canvasId);
        overlayCanvas = document.getElementById(overlayCanvasId);
        if (!canvas || !overlayCanvas) return;

        ctx = canvas.getContext('2d');
        overlayCtx = overlayCanvas.getContext('2d');

        const page = await pdfDoc.getPage(pageNum);
        const viewport = page.getViewport({ scale: scale });

        // 高解像度ディスプレイ対応（Retina等）
        const dpr = window.devicePixelRatio || 1;

        // キャンバスの内部解像度を上げる
        canvas.width = viewport.width * dpr;
        canvas.height = viewport.height * dpr;
        overlayCanvas.width = viewport.width * dpr;
        overlayCanvas.height = viewport.height * dpr;

        // CSSサイズは元のサイズに
        canvas.style.width = viewport.width + 'px';
        canvas.style.height = viewport.height + 'px';
        overlayCanvas.style.width = viewport.width + 'px';
        overlayCanvas.style.height = viewport.height + 'px';

        // コンテキストをリセットしてスケール
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        overlayCtx.setTransform(dpr, 0, 0, dpr, 0, 0);

        await page.render({
            canvasContext: ctx,
            viewport: viewport
        }).promise;

        redrawAnnotations();
        currentPage = pageNum;
    }

    /**
     * テキストの幅と高さを計算
     */
    function measureText(text, fontSize, fontFamily) {
        const family = fontFamily || currentFontFamily;
        const cssFont = family === 'mincho' ? 'serif' : 'sans-serif';

        // overlayCtxがない場合は一時的なcanvasを使用
        let ctx = overlayCtx;
        if (!ctx) {
            const tempCanvas = document.createElement('canvas');
            ctx = tempCanvas.getContext('2d');
        }

        ctx.font = `${fontSize * scale}px ${cssFont}`;
        const metrics = ctx.measureText(text);
        return {
            width: metrics.width / scale,
            height: fontSize * 1.2  // 行の高さの近似
        };
    }

    /**
     * テキスト注釈を追加
     */
    function addTextAnnotation(x, y, text) {
        const dims = measureText(text, currentFontSize, currentFontFamily);
        const annotation = {
            id: 'ann_' + (annotationIdCounter++),
            page: currentPage,
            x: x / scale,
            y: y / scale,
            text: text,
            fontSize: currentFontSize,
            fontFamily: currentFontFamily,
            color: currentColor,
            width: dims.width,
            height: dims.height
        };
        textAnnotations.push(annotation);
        redrawAnnotations();
        saveAnnotationsToStorage();
        return annotation;
    }

    /**
     * 座標位置にある注釈を取得
     */
    function getAnnotationAt(screenX, screenY) {
        const x = screenX / scale;
        const y = screenY / scale;

        // 現在のページの注釈のみを対象
        const pageAnnotations = textAnnotations.filter(a => a.page === currentPage);

        // ヒット判定のマージン（選択しやすくするため）
        const margin = 5;

        // 後から追加されたものが上に表示されるので、逆順でチェック
        for (let i = pageAnnotations.length - 1; i >= 0; i--) {
            const ann = pageAnnotations[i];
            // テキストのバウンディングボックスでヒット判定（マージン付き）
            // y座標はテキストのベースラインなので、上方向にheight分がテキスト領域
            if (x >= ann.x - margin && x <= ann.x + ann.width + margin &&
                y >= ann.y - ann.height - margin && y <= ann.y + margin) {
                return ann;
            }
        }
        return null;
    }

    /**
     * 注釈を選択
     */
    function selectAnnotation(id) {
        selectedId = id;
        redrawAnnotations();
        return selectedId;
    }

    /**
     * 選択解除
     */
    function deselectAll() {
        selectedId = null;
        redrawAnnotations();
    }

    /**
     * 選択中の注釈を削除
     */
    function deleteSelected() {
        if (!selectedId) return false;

        const idx = textAnnotations.findIndex(a => a.id === selectedId);
        if (idx !== -1) {
            textAnnotations.splice(idx, 1);
            selectedId = null;
            redrawAnnotations();
            saveAnnotationsToStorage();
            return true;
        }
        return false;
    }

    /**
     * 選択中の注釈を矢印キーで移動
     */
    function nudgeSelected(dx, dy) {
        if (!selectedId) return false;

        const ann = textAnnotations.find(a => a.id === selectedId);
        if (!ann) return false;

        ann.x += dx;
        ann.y += dy;
        redrawAnnotations();
        saveAnnotationsToStorage();
        return true;
    }

    /**
     * 選択中の注釈のテキストを取得
     */
    function getSelectedText() {
        if (!selectedId) return null;
        const ann = textAnnotations.find(a => a.id === selectedId);
        return ann ? ann.text : null;
    }

    /**
     * 選択中の注釈のテキストを更新
     */
    function updateSelectedText(newText) {
        if (!selectedId) return false;
        const ann = textAnnotations.find(a => a.id === selectedId);
        if (!ann) return false;

        ann.text = newText;
        // 幅を再計算（フォントファミリーも考慮）
        const dims = measureText(newText, ann.fontSize, ann.fontFamily);
        ann.width = dims.width;
        ann.height = dims.height;
        redrawAnnotations();
        saveAnnotationsToStorage();
        return true;
    }

    /**
     * 選択中の注釈のフォントサイズを変更
     */
    function updateSelectedFontSize(newSize) {
        if (!selectedId) return false;
        const ann = textAnnotations.find(a => a.id === selectedId);
        if (!ann) return false;

        ann.fontSize = newSize;
        // 幅を再計算
        const dims = measureText(ann.text, newSize, ann.fontFamily);
        ann.width = dims.width;
        ann.height = dims.height;
        redrawAnnotations();
        saveAnnotationsToStorage();
        return true;
    }

    /**
     * 選択中の注釈のフォントファミリーを変更
     */
    function updateSelectedFontFamily(newFamily) {
        if (!selectedId) return false;
        const ann = textAnnotations.find(a => a.id === selectedId);
        if (!ann) return false;

        ann.fontFamily = newFamily;
        // 幅を再計算
        const dims = measureText(ann.text, ann.fontSize, newFamily);
        ann.width = dims.width;
        ann.height = dims.height;
        redrawAnnotations();
        saveAnnotationsToStorage();
        return true;
    }

    /**
     * ドラッグ開始
     */
    function startDrag(screenX, screenY) {
        if (!selectedId) return false;

        const ann = textAnnotations.find(a => a.id === selectedId);
        if (!ann) return false;

        isDragging = true;
        dragStartX = screenX;
        dragStartY = screenY;
        dragOffsetX = screenX / scale - ann.x;
        dragOffsetY = screenY / scale - ann.y;
        return true;
    }

    /**
     * ドラッグ中の更新
     */
    function updateDrag(screenX, screenY) {
        if (!isDragging || !selectedId) return;

        const ann = textAnnotations.find(a => a.id === selectedId);
        if (!ann) return;

        ann.x = screenX / scale - dragOffsetX;
        ann.y = screenY / scale - dragOffsetY;
        redrawAnnotations();
    }

    /**
     * ドラッグ終了
     */
    function endDrag() {
        if (isDragging) {
            saveAnnotationsToStorage();
        }
        isDragging = false;
    }

    /**
     * ホバー状態を更新
     */
    function updateHover(screenX, screenY) {
        if (editMode !== 'select') {
            if (hoveredId !== null) {
                hoveredId = null;
                redrawAnnotations();
            }
            return null;
        }

        const ann = getAnnotationAt(screenX, screenY);
        const newHoveredId = ann ? ann.id : null;

        if (newHoveredId !== hoveredId) {
            hoveredId = newHoveredId;
            redrawAnnotations();
        }
        return hoveredId;
    }

    /**
     * 注釈を再描画
     */
    function redrawAnnotations() {
        if (!overlayCtx || !overlayCanvas) return;

        // dprでスケール済みなので論理サイズでクリア
        const dpr = window.devicePixelRatio || 1;
        overlayCtx.clearRect(0, 0, overlayCanvas.width / dpr, overlayCanvas.height / dpr);

        textAnnotations.filter(a => a.page === currentPage).forEach(annotation => {
            const screenX = annotation.x * scale;
            const screenY = annotation.y * scale;
            const screenWidth = annotation.width * scale;
            const screenHeight = annotation.height * scale;

            // 選択中の場合、背景を描画
            if (annotation.id === selectedId) {
                overlayCtx.fillStyle = 'rgba(33, 150, 243, 0.2)';
                overlayCtx.fillRect(
                    screenX - 2,
                    screenY - screenHeight - 2,
                    screenWidth + 4,
                    screenHeight + 4
                );
                overlayCtx.strokeStyle = '#2196F3';
                overlayCtx.lineWidth = 2;
                overlayCtx.strokeRect(
                    screenX - 2,
                    screenY - screenHeight - 2,
                    screenWidth + 4,
                    screenHeight + 4
                );
            } else if (annotation.id === hoveredId) {
                // ホバー中の場合、薄い枠を描画
                overlayCtx.strokeStyle = 'rgba(33, 150, 243, 0.5)';
                overlayCtx.lineWidth = 1;
                overlayCtx.strokeRect(
                    screenX - 2,
                    screenY - screenHeight - 2,
                    screenWidth + 4,
                    screenHeight + 4
                );
            }

            // テキストを描画（フォントファミリーを適用）
            const cssFont = annotation.fontFamily === 'mincho' ? 'serif' : 'sans-serif';
            overlayCtx.font = `${annotation.fontSize * scale}px ${cssFont}`;
            overlayCtx.fillStyle = annotation.color;
            overlayCtx.fillText(annotation.text, screenX, screenY);
        });
    }

    /**
     * 最後の注釈を削除
     */
    function undoLastAnnotation() {
        const pageAnnotations = textAnnotations.filter(a => a.page === currentPage);
        if (pageAnnotations.length > 0) {
            const lastIdx = textAnnotations.lastIndexOf(pageAnnotations[pageAnnotations.length - 1]);
            textAnnotations.splice(lastIdx, 1);
            if (selectedId && !textAnnotations.find(a => a.id === selectedId)) {
                selectedId = null;
            }
            redrawAnnotations();
            saveAnnotationsToStorage();
        }
    }

    /**
     * PDFが有効かチェック（%PDF-で始まるか）
     */
    function isValidPdf(bytes) {
        if (!bytes || bytes.length < 5) return false;
        // Check for PDF magic bytes: %PDF-
        return bytes[0] === 0x25 && bytes[1] === 0x50 &&
               bytes[2] === 0x44 && bytes[3] === 0x46 && bytes[4] === 0x2D;
    }

    /**
     * PDFを保存（テキスト追加済み）
     */
    async function savePdf() {
        console.log('savePdf called, annotations:', textAnnotations.length, textAnnotations);
        if (!pdfBytes || textAnnotations.length === 0) {
            console.log('No pdfBytes or no annotations, returning original');
            return pdfBytes;
        }

        // PDFが有効かチェック
        if (!isValidPdf(pdfBytes)) {
            throw new Error('PDFが読み込まれていないか、無効なPDFです。ページをリロードしてPDFを再度開いてください。');
        }

        const { PDFDocument, rgb } = PDFLib;
        const pdfDocLib = await PDFDocument.load(pdfBytes);

        // fontkitを登録（カスタムフォント埋め込みに必要）
        if (typeof fontkit !== 'undefined') {
            pdfDocLib.registerFontkit(fontkit);
        }

        // 使用するフォントファミリーを収集
        const usedFamilies = new Set(textAnnotations.map(a => a.fontFamily || 'gothic'));

        // 必要なフォントを読み込み・埋め込み
        const embeddedFonts = {};
        for (const family of usedFamilies) {
            const fontBytes = await getJapaneseFont(family);
            embeddedFonts[family] = await pdfDocLib.embedFont(fontBytes);
        }

        const pages = pdfDocLib.getPages();

        for (const annotation of textAnnotations) {
            if (annotation.page <= pages.length) {
                const page = pages[annotation.page - 1];
                const { width, height } = page.getSize();
                const rotation = page.getRotation().angle;

                // ページ回転を考慮した座標変換
                let pdfX, pdfY;
                if (rotation === 90) {
                    pdfX = annotation.y;
                    pdfY = annotation.x;
                } else if (rotation === 180) {
                    pdfX = width - annotation.x;
                    pdfY = annotation.y;
                } else if (rotation === 270) {
                    pdfX = height - annotation.y;
                    pdfY = width - annotation.x;
                } else {
                    // 0度または回転なし
                    pdfX = annotation.x;
                    pdfY = height - annotation.y;
                }

                const colorHex = annotation.color.replace('#', '');
                const r = parseInt(colorHex.substr(0, 2), 16) / 255;
                const g = parseInt(colorHex.substr(2, 2), 16) / 255;
                const b = parseInt(colorHex.substr(4, 2), 16) / 255;

                const fontFamily = annotation.fontFamily || 'gothic';
                const font = embeddedFonts[fontFamily];

                // テキストの回転角度（ページ回転の逆方向）
                const textRotation = rotation ? (360 - rotation) * Math.PI / 180 : 0;

                console.log('Drawing text:', annotation.text, 'at', pdfX, pdfY, 'rotation:', rotation, 'textRotation:', textRotation);
                page.drawText(annotation.text, {
                    x: pdfX,
                    y: pdfY,
                    size: annotation.fontSize,
                    font: font,
                    color: rgb(r, g, b),
                    rotate: PDFLib.degrees(rotation)
                });
            }
        }

        const modifiedPdfBytes = await pdfDocLib.save();
        return modifiedPdfBytes;
    }

    /**
     * PDFをダウンロード
     */
    async function downloadPdf(filename) {
        const pdfBytesModified = await savePdf();
        const blob = new Blob([pdfBytesModified], { type: 'application/pdf' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename || 'edited.pdf';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }

    /**
     * PDFをGoogle Driveにアップロード
     * @param {string} gasUrl - GASのWebアプリURL
     * @param {string} originalFileId - 元ファイルのID
     * @param {string} newFileName - 新しいファイル名（別名保存時）
     * @param {boolean} overwrite - 上書きするかどうか
     * @returns {Promise<Object>} アップロード結果
     */
    async function uploadPdfToDrive(gasUrl, originalFileId, newFileName, overwrite) {
        const pdfBytesModified = await savePdf();
        if (!pdfBytesModified) {
            throw new Error('PDF data is empty');
        }

        // Uint8ArrayをBase64に変換
        let binary = '';
        const bytes = new Uint8Array(pdfBytesModified);
        for (let i = 0; i < bytes.byteLength; i++) {
            binary += String.fromCharCode(bytes[i]);
        }
        const base64 = btoa(binary);

        // GASにPOSTリクエスト（Content-Type指定なしでプリフライト回避）
        const response = await fetch(gasUrl, {
            method: 'POST',
            redirect: 'follow',
            body: JSON.stringify({
                action: 'uploadPdf',
                base64: base64,
                originalFileId: originalFileId,
                newFileName: newFileName,
                overwrite: overwrite
            })
        });

        const result = await response.json();
        if (result.error) {
            throw new Error(result.error);
        }
        return result;
    }

    /**
     * 設定変更
     */
    function setFontSize(size) {
        currentFontSize = size;
    }

    function setFontFamily(family) {
        currentFontFamily = family;
    }

    function setColor(color) {
        currentColor = color;
    }

    function setScale(newScale) {
        scale = newScale;
    }

    function setEditMode(mode) {
        editMode = mode;
        if (mode === 'add') {
            selectedId = null;
            hoveredId = null;
            redrawAnnotations();
        }
        return editMode;
    }

    function getEditMode() {
        return editMode;
    }

    function getSelectedId() {
        return selectedId;
    }

    function getState() {
        return {
            currentPage,
            totalPages,
            annotationCount: textAnnotations.length,
            fontSize: currentFontSize,
            color: currentColor,
            editMode: editMode,
            selectedId: selectedId
        };
    }

    function nextPage() {
        if (currentPage < totalPages) {
            return currentPage + 1;
        }
        return currentPage;
    }

    function prevPage() {
        if (currentPage > 1) {
            return currentPage - 1;
        }
        return currentPage;
    }

    return {
        loadPdf,
        loadPdfFromBase64,
        renderPage,
        addTextAnnotation,
        undoLastAnnotation,
        clearAnnotations,
        savePdf,
        downloadPdf,
        uploadPdfToDrive,
        setFontSize,
        setFontFamily,
        setColor,
        setScale,
        setEditMode,
        getEditMode,
        getSelectedId,
        getState,
        nextPage,
        prevPage,
        // 選択・移動・削除用
        getAnnotationAt,
        selectAnnotation,
        deselectAll,
        deleteSelected,
        nudgeSelected,
        getSelectedText,
        updateSelectedText,
        updateSelectedFontSize,
        updateSelectedFontFamily,
        startDrag,
        updateDrag,
        endDrag,
        updateHover
    };
})();

/**
 * PDFプリフェッチ機能 (IndexedDB版)
 * GASからPDFを取得してIndexedDBに永続保存
 */

// IndexedDB設定
const PDF_CACHE_DB_NAME = 'PdfCacheDB';
const PDF_CACHE_STORE_NAME = 'pdfCache';
const PDF_CACHE_DB_VERSION = 1;
const PDF_CACHE_TTL = 24 * 60 * 60 * 1000;  // 24時間

// フェッチ中のPromise（メモリ内のみ）
window.__pdfCachePending = window.__pdfCachePending || {};

/**
 * IndexedDBを開く
 */
function openPdfCacheDB() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(PDF_CACHE_DB_NAME, PDF_CACHE_DB_VERSION);

        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve(request.result);

        request.onupgradeneeded = (event) => {
            const db = event.target.result;
            if (!db.objectStoreNames.contains(PDF_CACHE_STORE_NAME)) {
                db.createObjectStore(PDF_CACHE_STORE_NAME, { keyPath: 'fileId' });
            }
        };
    });
}

/**
 * IndexedDBからPDFを取得
 */
async function getFromIndexedDB(fileId) {
    try {
        const db = await openPdfCacheDB();
        return new Promise((resolve, reject) => {
            const tx = db.transaction(PDF_CACHE_STORE_NAME, 'readonly');
            const store = tx.objectStore(PDF_CACHE_STORE_NAME);
            const request = store.get(fileId);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => {
                const entry = request.result;
                if (!entry) {
                    resolve(null);
                    return;
                }
                // TTL チェック
                if (Date.now() - entry.timestamp > PDF_CACHE_TTL) {
                    // 期限切れ: 削除して null 返却
                    deleteFromIndexedDB(fileId);
                    resolve(null);
                    return;
                }
                resolve(entry.base64);
            };
        });
    } catch (e) {
        console.error('[IndexedDB] getFromIndexedDB error:', e);
        return null;
    }
}

/**
 * IndexedDBにPDFを保存
 */
async function saveToIndexedDB(fileId, base64) {
    try {
        const db = await openPdfCacheDB();
        return new Promise((resolve, reject) => {
            const tx = db.transaction(PDF_CACHE_STORE_NAME, 'readwrite');
            const store = tx.objectStore(PDF_CACHE_STORE_NAME);
            const request = store.put({
                fileId: fileId,
                base64: base64,
                timestamp: Date.now()
            });

            request.onerror = () => reject(request.error);
            request.onsuccess = () => resolve(true);
        });
    } catch (e) {
        console.error('[IndexedDB] saveToIndexedDB error:', e);
        return false;
    }
}

/**
 * IndexedDBからPDFを削除
 */
async function deleteFromIndexedDB(fileId) {
    try {
        const db = await openPdfCacheDB();
        return new Promise((resolve, reject) => {
            const tx = db.transaction(PDF_CACHE_STORE_NAME, 'readwrite');
            const store = tx.objectStore(PDF_CACHE_STORE_NAME);
            const request = store.delete(fileId);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => resolve(true);
        });
    } catch (e) {
        console.error('[IndexedDB] deleteFromIndexedDB error:', e);
        return false;
    }
}

/**
 * PDFをプリフェッチ（IndexedDB版）
 */
window.prefetchPdf = async function(fileId, gasUrl) {
    // 既にフェッチ中なら、そのPromiseを返す
    if (window.__pdfCachePending[fileId]) {
        console.log('[prefetch] PDF already fetching:', fileId);
        return window.__pdfCachePending[fileId];
    }

    // IndexedDBにキャッシュがあればスキップ
    const cached = await getFromIndexedDB(fileId);
    if (cached) {
        console.log('[prefetch] PDF found in IndexedDB:', fileId);
        return true;
    }

    // フェッチを開始してPromiseを保存
    const fetchPromise = (async () => {
        try {
            console.log('[prefetch] Fetching PDF:', fileId);
            const response = await fetch(`${gasUrl}?action=fetchPdf&fileId=${encodeURIComponent(fileId)}`);
            if (!response.ok) throw new Error('PDF fetch failed');
            const data = await response.json();
            if (data.error) throw new Error(data.error);
            if (!data.base64) throw new Error('No PDF data');

            // IndexedDBに保存
            await saveToIndexedDB(fileId, data.base64);
            console.log('[prefetch] PDF cached to IndexedDB:', fileId);
            return true;
        } catch (e) {
            console.error('[prefetch] Error:', e);
            return false;
        } finally {
            delete window.__pdfCachePending[fileId];
        }
    })();

    window.__pdfCachePending[fileId] = fetchPromise;
    return fetchPromise;
};

// フェッチ中のPromiseを取得（iframe側から待機用）
window.getPdfCachePending = function(fileId) {
    return window.__pdfCachePending[fileId] || null;
};

// キャッシュからBase64を取得（非同期版）
window.getCachedPdfBase64Async = async function(fileId) {
    return await getFromIndexedDB(fileId);
};

// キャッシュにPDFを保存
window.setCachedPdfBase64 = async function(fileId, base64) {
    return await saveToIndexedDB(fileId, base64);
};

// キャッシュをクリア
window.clearPdfCache = async function() {
    try {
        const db = await openPdfCacheDB();
        return new Promise((resolve, reject) => {
            const tx = db.transaction(PDF_CACHE_STORE_NAME, 'readwrite');
            const store = tx.objectStore(PDF_CACHE_STORE_NAME);
            const request = store.clear();

            request.onerror = () => reject(request.error);
            request.onsuccess = () => resolve(true);
        });
    } catch (e) {
        console.error('[IndexedDB] clearPdfCache error:', e);
        return false;
    }
};

// ============================================
// APIキー暗号化/復号（固定キー）
// ============================================

const API_KEY_FIXED_PASSWORD = 'SekouTaisei2024!AppKey#Encrypt';
const API_KEY_STORAGE_KEY = 'sekou_taisei_api_key';

/**
 * PBKDF2でキーを導出
 */
async function deriveApiKeyEncryptionKey(salt) {
    const encoder = new TextEncoder();
    const passwordBuffer = encoder.encode(API_KEY_FIXED_PASSWORD);
    const passwordKey = await crypto.subtle.importKey('raw', passwordBuffer, 'PBKDF2', false, ['deriveKey']);
    return crypto.subtle.deriveKey(
        { name: 'PBKDF2', salt: salt.buffer, iterations: 100000, hash: 'SHA-256' },
        passwordKey,
        { name: 'AES-GCM', length: 256 },
        false,
        ['encrypt', 'decrypt']
    );
}

/**
 * APIキーを暗号化（スプレッドシート保存用）
 */
window.encryptApiKey = async function(apiKey) {
    try {
        const encoder = new TextEncoder();
        const data = encoder.encode(apiKey);
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const iv = crypto.getRandomValues(new Uint8Array(12));
        const key = await deriveApiKeyEncryptionKey(salt);
        const encryptedBuffer = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: iv.buffer }, key, data);

        return JSON.stringify({
            encrypted: btoa(String.fromCharCode(...new Uint8Array(encryptedBuffer))),
            iv: btoa(String.fromCharCode(...iv)),
            salt: btoa(String.fromCharCode(...salt))
        });
    } catch (e) {
        console.error('[encryptApiKey] error:', e);
        return null;
    }
};

/**
 * 暗号化されたAPIキーを復号
 */
window.decryptApiKey = async function(encryptedJson) {
    try {
        const { encrypted, iv, salt } = JSON.parse(encryptedJson);
        const encryptedBuffer = Uint8Array.from(atob(encrypted), c => c.charCodeAt(0));
        const ivBytes = Uint8Array.from(atob(iv), c => c.charCodeAt(0));
        const saltBytes = Uint8Array.from(atob(salt), c => c.charCodeAt(0));

        const key = await deriveApiKeyEncryptionKey(saltBytes);
        const decryptedBuffer = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: ivBytes.buffer }, key, encryptedBuffer);

        return new TextDecoder().decode(decryptedBuffer);
    } catch (e) {
        console.error('[decryptApiKey] error:', e);
        return null;
    }
};

/**
 * スプレッドシートから読み込んだ暗号化APIキーを復号してセット
 */
window.loadEncryptedApiKey = async function(encryptedData) {
    if (!encryptedData) {
        console.log('[loadEncryptedApiKey] No encrypted data');
        return false;
    }

    try {
        const decrypted = await window.decryptApiKey(encryptedData);
        if (decrypted && decrypted.startsWith('AIza')) {
            localStorage.setItem(API_KEY_STORAGE_KEY, decrypted);
            sessionStorage.setItem(API_KEY_STORAGE_KEY, decrypted);
            console.log('[loadEncryptedApiKey] API key loaded successfully');
            return true;
        }
        console.log('[loadEncryptedApiKey] Invalid key format');
        return false;
    } catch (e) {
        console.error('[loadEncryptedApiKey] error:', e);
        return false;
    }
};

/**
 * 現在のAPIキーを暗号化してスプレッドシートに保存
 */
window.saveApiKeyToSpreadsheet = async function(gasUrl) {
    const apiKey = localStorage.getItem(API_KEY_STORAGE_KEY);
    if (!apiKey) {
        console.log('[saveApiKeyToSpreadsheet] No API key to save');
        return false;
    }

    try {
        const encryptedData = await window.encryptApiKey(apiKey);
        if (!encryptedData) {
            throw new Error('Encryption failed');
        }

        const response = await fetch(gasUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'text/plain' },
            body: JSON.stringify({
                action: 'saveSettings',
                settings: {
                    encryptedApiKey: encryptedData
                }
            })
        });

        const result = await response.json();
        console.log('[saveApiKeyToSpreadsheet] Result:', result);
        return result.success === true;
    } catch (e) {
        console.error('[saveApiKeyToSpreadsheet] error:', e);
        return false;
    }
};

/**
 * APIキーが設定されているか確認
 */
window.hasApiKey = function() {
    const key = localStorage.getItem(API_KEY_STORAGE_KEY);
    return key && key.startsWith('AIza');
};
