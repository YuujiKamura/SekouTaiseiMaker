/**
 * PDFキャッシュサービス
 * 親ウィンドウ（Leptos側）のグローバルキャッシュにアクセス
 * プレーンオブジェクト + Base64文字列を使用（iframe間で共有可能）
 */

// 親ウィンドウのグローバル関数・変数型定義
declare global {
  interface Window {
    __pdfCacheData?: Record<string, { base64: string; timestamp: number }>;
    __pdfCachePending?: Record<string, Promise<boolean>>;
    getCachedPdfBase64?: (fileId: string) => string | null;
    getPdfCachePending?: (fileId: string) => Promise<boolean> | null;
  }
}

function getParentWindow(): Window | null {
  try {
    if (window.parent && window.parent !== window) {
      // 親ウィンドウにアクセス可能か確認
      const cache = window.parent.__pdfCacheData;
      if (typeof cache !== 'undefined') {
        return window.parent;
      }
    }
  } catch {
    // Cross-origin の場合はアクセス不可
  }
  return null;
}

/**
 * フェッチ中のPromiseを取得
 */
export function getPdfCachePending(fileId: string): Promise<boolean> | null {
  const parent = getParentWindow();
  if (!parent) return null;

  if (typeof parent.getPdfCachePending === 'function') {
    return parent.getPdfCachePending(fileId);
  }

  return parent.__pdfCachePending?.[fileId] || null;
}

/**
 * キャッシュからPDF（Base64）を取得
 */
export function getCachedPdfBase64(fileId: string): string | null {
  const parent = getParentWindow();
  if (!parent) return null;

  // 親ウィンドウの関数を使用
  if (typeof parent.getCachedPdfBase64 === 'function') {
    return parent.getCachedPdfBase64(fileId);
  }

  // フォールバック: 直接アクセス
  const entry = parent.__pdfCacheData?.[fileId];
  if (!entry) return null;

  // 10分以上経過したら無効
  const CACHE_TTL = 10 * 60 * 1000;
  if (Date.now() - entry.timestamp > CACHE_TTL) {
    delete parent.__pdfCacheData![fileId];
    return null;
  }

  return entry.base64;
}

/**
 * キャッシュからPDFを取得してArrayBufferに変換
 * フェッチ中なら待機する
 */
export async function getCachedPdfAsync(fileId: string): Promise<ArrayBuffer | null> {
  // まずキャッシュをチェック
  let base64 = getCachedPdfBase64(fileId);

  // なければ、フェッチ中かチェックして待機
  if (!base64) {
    const pending = getPdfCachePending(fileId);
    if (pending) {
      console.log('[pdfCache] Waiting for prefetch:', fileId);
      await pending;
      base64 = getCachedPdfBase64(fileId);
    }
  }

  if (!base64) return null;

  // Base64 → ArrayBuffer
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

/**
 * キャッシュからPDFを取得してArrayBufferに変換（同期版）
 */
export function getCachedPdf(fileId: string): ArrayBuffer | null {
  const base64 = getCachedPdfBase64(fileId);
  if (!base64) return null;

  // Base64 → ArrayBuffer
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

/**
 * キャッシュにPDFを保存（Base64形式）
 */
export function setCachedPdfBase64(fileId: string, base64: string): void {
  const parent = getParentWindow();
  if (!parent) return;

  if (!parent.__pdfCacheData) {
    parent.__pdfCacheData = {};
  }

  parent.__pdfCacheData[fileId] = {
    base64,
    timestamp: Date.now(),
  };
}

/**
 * キャッシュにPDFを保存（ArrayBuffer形式）
 */
export function setCachedPdf(fileId: string, data: ArrayBuffer): void {
  // ArrayBuffer → Base64
  const bytes = new Uint8Array(data);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  const base64 = btoa(binary);
  setCachedPdfBase64(fileId, base64);
}

/**
 * キャッシュをクリア
 */
export function clearPdfCache(): void {
  const parent = getParentWindow();
  if (parent && parent.__pdfCacheData) {
    parent.__pdfCacheData = {};
  }
}
