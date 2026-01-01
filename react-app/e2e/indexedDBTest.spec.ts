import { test, expect } from '@playwright/test';

const BASE_URL = 'http://localhost:8080';

test.describe('IndexedDB PDF Cache', () => {
  test('IndexedDB is accessible and can store data', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForTimeout(2000);

    // IndexedDBが使えるか確認
    const canUseIndexedDB = await page.evaluate(async () => {
      try {
        const request = indexedDB.open('PdfCacheDB', 1);
        return new Promise((resolve) => {
          request.onerror = () => resolve({ success: false, error: 'Failed to open' });
          request.onsuccess = () => {
            const db = request.result;
            const hasStore = db.objectStoreNames.contains('pdfCache');
            db.close();
            resolve({ success: true, hasStore });
          };
          request.onupgradeneeded = (event) => {
            const db = (event.target as IDBOpenDBRequest).result;
            if (!db.objectStoreNames.contains('pdfCache')) {
              db.createObjectStore('pdfCache', { keyPath: 'fileId' });
            }
          };
        });
      } catch (e) {
        return { success: false, error: String(e) };
      }
    });

    console.log('=== IndexedDB Check ===');
    console.log('Result:', JSON.stringify(canUseIndexedDB, null, 2));
    expect(canUseIndexedDB).toHaveProperty('success', true);
  });

  test('Can save and retrieve PDF from IndexedDB', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForTimeout(2000);

    // テストデータを保存して取得
    const testResult = await page.evaluate(async () => {
      const DB_NAME = 'PdfCacheDB';
      const STORE_NAME = 'pdfCache';
      const TEST_FILE_ID = 'test-file-123';
      const TEST_BASE64 = 'dGVzdCBwZGYgZGF0YQ=='; // "test pdf data" in base64

      // DBを開く
      const openDB = (): Promise<IDBDatabase> => {
        return new Promise((resolve, reject) => {
          const request = indexedDB.open(DB_NAME, 1);
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve(request.result);
          request.onupgradeneeded = (event) => {
            const db = (event.target as IDBOpenDBRequest).result;
            if (!db.objectStoreNames.contains(STORE_NAME)) {
              db.createObjectStore(STORE_NAME, { keyPath: 'fileId' });
            }
          };
        });
      };

      try {
        const db = await openDB();

        // 保存
        await new Promise<void>((resolve, reject) => {
          const tx = db.transaction(STORE_NAME, 'readwrite');
          const store = tx.objectStore(STORE_NAME);
          const request = store.put({
            fileId: TEST_FILE_ID,
            base64: TEST_BASE64,
            timestamp: Date.now()
          });
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve();
        });

        // 取得
        const retrieved = await new Promise<any>((resolve, reject) => {
          const tx = db.transaction(STORE_NAME, 'readonly');
          const store = tx.objectStore(STORE_NAME);
          const request = store.get(TEST_FILE_ID);
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve(request.result);
        });

        // 全件取得
        const allEntries = await new Promise<any[]>((resolve, reject) => {
          const tx = db.transaction(STORE_NAME, 'readonly');
          const store = tx.objectStore(STORE_NAME);
          const request = store.getAll();
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve(request.result);
        });

        db.close();

        return {
          success: true,
          saved: true,
          retrieved: retrieved ? {
            fileId: retrieved.fileId,
            base64: retrieved.base64,
            hasTimestamp: !!retrieved.timestamp
          } : null,
          totalEntries: allEntries.length,
          allFileIds: allEntries.map(e => e.fileId)
        };
      } catch (e) {
        return { success: false, error: String(e) };
      }
    });

    console.log('=== IndexedDB Save/Retrieve Test ===');
    console.log('Result:', JSON.stringify(testResult, null, 2));

    expect(testResult).toHaveProperty('success', true);
    expect(testResult).toHaveProperty('saved', true);
    expect(testResult.retrieved).not.toBeNull();
    expect(testResult.retrieved?.fileId).toBe('test-file-123');
  });

  test('Check existing cached PDFs in IndexedDB', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForTimeout(2000);

    const cacheStatus = await page.evaluate(async () => {
      const DB_NAME = 'PdfCacheDB';
      const STORE_NAME = 'pdfCache';

      try {
        const request = indexedDB.open(DB_NAME, 1);
        const db = await new Promise<IDBDatabase>((resolve, reject) => {
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve(request.result);
          request.onupgradeneeded = (event) => {
            const db = (event.target as IDBOpenDBRequest).result;
            if (!db.objectStoreNames.contains(STORE_NAME)) {
              db.createObjectStore(STORE_NAME, { keyPath: 'fileId' });
            }
          };
        });

        const entries = await new Promise<any[]>((resolve, reject) => {
          const tx = db.transaction(STORE_NAME, 'readonly');
          const store = tx.objectStore(STORE_NAME);
          const request = store.getAll();
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve(request.result);
        });

        db.close();

        return {
          success: true,
          count: entries.length,
          entries: entries.map(e => ({
            fileId: e.fileId,
            size: e.base64 ? Math.round(e.base64.length / 1024) + 'KB' : 'N/A',
            timestamp: e.timestamp ? new Date(e.timestamp).toISOString() : 'N/A',
            age: e.timestamp ? Math.round((Date.now() - e.timestamp) / 1000 / 60) + ' min ago' : 'N/A'
          }))
        };
      } catch (e) {
        return { success: false, error: String(e) };
      }
    });

    console.log('=== Existing IndexedDB Cache ===');
    console.log('Result:', JSON.stringify(cacheStatus, null, 2));

    expect(cacheStatus).toHaveProperty('success', true);
  });
});
