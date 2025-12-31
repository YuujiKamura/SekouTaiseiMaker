/**
 * PDF Editor Module
 * PDFにテキストを追加して保存する機能
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
    let currentFontFamily = 'HeiseiMin-W3'; // 日本語対応フォント
    let currentColor = '#000000';

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
                    const loadingTask = pdfjsLib.getDocument({ data: pdfBytes });
                    pdfDoc = await loadingTask.promise;
                    totalPages = pdfDoc.numPages;
                    currentPage = 1;
                    textAnnotations = [];
                    resolve({ totalPages: totalPages });
                } catch (err) {
                    reject(err);
                }
            };
            reader.onerror = reject;
            reader.readAsArrayBuffer(file);
        });
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

        canvas.width = viewport.width;
        canvas.height = viewport.height;
        overlayCanvas.width = viewport.width;
        overlayCanvas.height = viewport.height;

        await page.render({
            canvasContext: ctx,
            viewport: viewport
        }).promise;

        // テキスト注釈を再描画
        redrawAnnotations();

        currentPage = pageNum;
    }

    /**
     * テキスト注釈を追加
     */
    function addTextAnnotation(x, y, text) {
        const annotation = {
            page: currentPage,
            x: x / scale,  // PDF座標に変換
            y: y / scale,
            text: text,
            fontSize: currentFontSize,
            fontFamily: currentFontFamily,
            color: currentColor
        };
        textAnnotations.push(annotation);
        redrawAnnotations();
        return annotation;
    }

    /**
     * 注釈を再描画
     */
    function redrawAnnotations() {
        if (!overlayCtx || !overlayCanvas) return;

        overlayCtx.clearRect(0, 0, overlayCanvas.width, overlayCanvas.height);

        textAnnotations.filter(a => a.page === currentPage).forEach(annotation => {
            overlayCtx.font = `${annotation.fontSize * scale}px sans-serif`;
            overlayCtx.fillStyle = annotation.color;
            overlayCtx.fillText(annotation.text, annotation.x * scale, annotation.y * scale);
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
            redrawAnnotations();
        }
    }

    /**
     * PDFを保存（テキスト追加済み）
     */
    async function savePdf() {
        if (!pdfBytes || textAnnotations.length === 0) {
            // 注釈がない場合は元のPDFをそのまま返す
            return pdfBytes;
        }

        const { PDFDocument, rgb, StandardFonts } = PDFLib;
        const pdfDocLib = await PDFDocument.load(pdfBytes);

        // 標準フォントを使用（日本語は制限あり）
        const font = await pdfDocLib.embedFont(StandardFonts.Helvetica);

        const pages = pdfDocLib.getPages();

        for (const annotation of textAnnotations) {
            if (annotation.page <= pages.length) {
                const page = pages[annotation.page - 1];
                const { height } = page.getSize();

                // PDF座標系はY軸が下から上なので変換
                const pdfY = height - annotation.y;

                // 色をRGBに変換
                const colorHex = annotation.color.replace('#', '');
                const r = parseInt(colorHex.substr(0, 2), 16) / 255;
                const g = parseInt(colorHex.substr(2, 2), 16) / 255;
                const b = parseInt(colorHex.substr(4, 2), 16) / 255;

                page.drawText(annotation.text, {
                    x: annotation.x,
                    y: pdfY,
                    size: annotation.fontSize,
                    font: font,
                    color: rgb(r, g, b)
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

    function getState() {
        return {
            currentPage,
            totalPages,
            annotationCount: textAnnotations.length,
            fontSize: currentFontSize,
            color: currentColor
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
        renderPage,
        addTextAnnotation,
        undoLastAnnotation,
        savePdf,
        downloadPdf,
        setFontSize,
        setFontFamily,
        setColor,
        setScale,
        getState,
        nextPage,
        prevPage
    };
})();
