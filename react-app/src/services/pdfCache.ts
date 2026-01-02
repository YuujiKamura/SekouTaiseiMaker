/**
 * PDFキャッシュサービス (IndexedDB版)
 * 親ウィンドウと同じIndexedDBにアクセスして永続キャッシュを実現
 */

// IndexedDB設定（pdf-editor.jsと同じ設定）
const PDF_CACHE_DB_NAME = 'PdfCacheDB';
const PDF_CACHE_STORE_NAME = 'pdfCache';
const PDF_CACHE_DB_VERSION = 1;
const PDF_CACHE_TTL = 24 * 60 * 60 * 1000;  // 24時間

// 親ウィンドウのグローバル関数・変数型定義
declare global {
  interface Window {
    __pdfCachePending?: Record<string, Promise<boolean>>;
    getPdfCachePending?: (fileId: string) => Promise<boolean> | null;
  }
}

/**
 * IndexedDBを開く
 */
function openPdfCacheDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(PDF_CACHE_DB_NAME, PDF_CACHE_DB_VERSION);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains(PDF_CACHE_STORE_NAME)) {
        db.createObjectStore(PDF_CACHE_STORE_NAME, { keyPath: 'fileId' });
      }
    };
  });
}

interface CacheEntry {
  fileId: string;
  base64: string;
  timestamp: number;
  modifiedTime?: string;  // Google DriveのmodifiedTime
}

/**
 * IndexedDBからキャッシュエントリを取得
 */
async function getEntryFromIndexedDB(fileId: string): Promise<CacheEntry | null> {
  try {
    const db = await openPdfCacheDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(PDF_CACHE_STORE_NAME, 'readonly');
      const store = tx.objectStore(PDF_CACHE_STORE_NAME);
      const request = store.get(fileId);

      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const entry = request.result as CacheEntry | undefined;
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
        resolve(entry);
      };
    });
  } catch (e) {
    console.error('[pdfCache] getEntryFromIndexedDB error:', e);
    return null;
  }
}

/**
 * IndexedDBからPDFを取得（Base64形式）
 */
async function getFromIndexedDB(fileId: string): Promise<string | null> {
  const entry = await getEntryFromIndexedDB(fileId);
  return entry?.base64 || null;
}

/**
 * IndexedDBにPDFを保存
 */
async function saveToIndexedDB(fileId: string, base64: string, modifiedTime?: string): Promise<boolean> {
  try {
    const db = await openPdfCacheDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(PDF_CACHE_STORE_NAME, 'readwrite');
      const store = tx.objectStore(PDF_CACHE_STORE_NAME);
      const entry: CacheEntry = {
        fileId: fileId,
        base64: base64,
        timestamp: Date.now(),
        modifiedTime: modifiedTime
      };
      const request = store.put(entry);

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve(true);
    });
  } catch (e) {
    console.error('[pdfCache] saveToIndexedDB error:', e);
    return false;
  }
}

/**
 * IndexedDBからPDFを削除
 */
async function deleteFromIndexedDB(fileId: string): Promise<boolean> {
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
    console.error('[pdfCache] deleteFromIndexedDB error:', e);
    return false;
  }
}

/**
 * 親ウィンドウのフェッチ中Promiseを取得
 */
function getParentPending(fileId: string): Promise<boolean> | null {
  try {
    if (window.parent && window.parent !== window) {
      if (typeof window.parent.getPdfCachePending === 'function') {
        return window.parent.getPdfCachePending(fileId);
      }
      return window.parent.__pdfCachePending?.[fileId] || null;
    }
  } catch {
    // Cross-origin の場合はアクセス不可
  }
  return null;
}

/**
 * キャッシュからPDF（Base64）を取得
 */
export async function getCachedPdfBase64(fileId: string): Promise<string | null> {
  return await getFromIndexedDB(fileId);
}

/**
 * キャッシュからPDFを取得してArrayBufferに変換
 * フェッチ中なら待機する
 */
export async function getCachedPdfAsync(fileId: string): Promise<ArrayBuffer | null> {
  // まずIndexedDBをチェック
  let base64 = await getFromIndexedDB(fileId);

  // なければ、フェッチ中かチェックして待機
  if (!base64) {
    const pending = getParentPending(fileId);
    if (pending) {
      console.log('[pdfCache] Waiting for prefetch:', fileId);
      await pending;
      base64 = await getFromIndexedDB(fileId);
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
 * キャッシュからPDFを取得してArrayBufferに変換（同期版は非推奨、非同期版を使用）
 */
export async function getCachedPdf(fileId: string): Promise<ArrayBuffer | null> {
  return await getCachedPdfAsync(fileId);
}

/**
 * キャッシュにPDFを保存（Base64形式）
 */
export async function setCachedPdfBase64(fileId: string, base64: string, modifiedTime?: string): Promise<void> {
  await saveToIndexedDB(fileId, base64, modifiedTime);
}

/**
 * キャッシュにPDFを保存（ArrayBuffer形式）
 */
export async function setCachedPdf(fileId: string, data: ArrayBuffer, modifiedTime?: string): Promise<void> {
  // ArrayBuffer → Base64
  const bytes = new Uint8Array(data);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  const base64 = btoa(binary);
  await saveToIndexedDB(fileId, base64, modifiedTime);
}

/**
 * キャッシュが有効か確認（modifiedTimeで比較）
 * キャッシュのmodifiedTimeがGASのmodifiedTime以上なら有効
 * （編集保存後はキャッシュの方が新しくなる）
 * @returns キャッシュが有効ならtrue、無効または存在しなければfalse
 */
export async function isCacheValid(fileId: string, currentModifiedTime: string): Promise<boolean> {
  const entry = await getEntryFromIndexedDB(fileId);
  if (!entry) return false;
  if (!entry.modifiedTime) return false;  // modifiedTimeがない古いキャッシュは無効
  // キャッシュのmodifiedTimeがGASのmodifiedTime以上なら有効
  const cacheTime = new Date(entry.modifiedTime).getTime();
  const serverTime = new Date(currentModifiedTime).getTime();
  console.log('[pdfCache] Cache time:', entry.modifiedTime, 'Server time:', currentModifiedTime);
  return cacheTime >= serverTime;
}

/**
 * キャッシュのmodifiedTimeを取得
 */
export async function getCachedModifiedTime(fileId: string): Promise<string | null> {
  const entry = await getEntryFromIndexedDB(fileId);
  return entry?.modifiedTime || null;
}

/**
 * 特定のキャッシュを削除
 */
export async function invalidateCache(fileId: string): Promise<void> {
  await deleteFromIndexedDB(fileId);
}

/**
 * キャッシュをクリア
 */
export async function clearPdfCache(): Promise<void> {
  try {
    const db = await openPdfCacheDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(PDF_CACHE_STORE_NAME, 'readwrite');
      const store = tx.objectStore(PDF_CACHE_STORE_NAME);
      const request = store.clear();

      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve();
    });
  } catch (e) {
    console.error('[pdfCache] clearPdfCache error:', e);
  }
}
