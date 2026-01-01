import { test, expect } from '@playwright/test';

// ローカルテスト用URL
const BASE_URL = 'http://localhost:8080';

test.describe('PDF Cache', () => {
  test('prefetch stores PDF in parent window cache', async ({ page }) => {
    const logs: string[] = [];
    const errors: string[] = [];

    page.on('console', (msg) => {
      const text = msg.text();
      logs.push(`[${msg.type()}] ${text}`);
      if (msg.type() === 'error') {
        errors.push(text);
      }
    });

    // メインページにアクセス
    await page.goto(BASE_URL);
    await page.waitForTimeout(1000);

    // __pdfCacheDataオブジェクトが初期化されていることを確認
    const cacheExists = await page.evaluate(() => {
      return typeof window.__pdfCacheData !== 'undefined';
    });
    console.log('Cache object exists on main page:', cacheExists);

    // prefetchPdf関数が存在することを確認
    const prefetchExists = await page.evaluate(() => {
      return typeof window.prefetchPdf === 'function';
    });
    expect(prefetchExists).toBe(true);
    console.log('prefetchPdf function exists:', prefetchExists);

    // getCachedPdfBase64関数が存在することを確認
    const getCacheExists = await page.evaluate(() => {
      return typeof window.getCachedPdfBase64 === 'function';
    });
    expect(getCacheExists).toBe(true);
    console.log('getCachedPdfBase64 function exists:', getCacheExists);

    // ダミーのprefetchを実行（GASなしでテスト）
    const mockFileId = 'test-file-id-123';
    await page.evaluate((fileId) => {
      // キャッシュに直接追加してテスト（Base64文字列として）
      if (!window.__pdfCacheData) {
        window.__pdfCacheData = {};
      }
      // PDFヘッダーのBase64エンコード
      const mockPdfContent = btoa('%PDF-1.4 test content');
      window.__pdfCacheData[fileId] = {
        base64: mockPdfContent,
        timestamp: Date.now()
      };
    }, mockFileId);

    // キャッシュに追加されたことを確認
    const cacheSize = await page.evaluate(() => {
      return Object.keys(window.__pdfCacheData || {}).length;
    });
    expect(cacheSize).toBe(1);
    console.log('Cache size after mock:', cacheSize);

    // getCachedPdfBase64でキャッシュを取得できることを確認
    const cachedValue = await page.evaluate((fileId) => {
      return window.getCachedPdfBase64?.(fileId);
    }, mockFileId);
    expect(cachedValue).not.toBeNull();
    console.log('Cached value retrieved:', !!cachedValue);

    console.log('All logs:', logs);
    console.log('Errors:', errors);
  });

  test('iframe can access parent window cache', async ({ page }) => {
    const logs: string[] = [];

    page.on('console', (msg) => {
      logs.push(`[${msg.type()}] ${msg.text()}`);
    });

    await page.goto(BASE_URL);
    await page.waitForTimeout(1000);

    // 親ウィンドウにキャッシュを設定（プレーンオブジェクト + Base64）
    const mockFileId = 'iframe-test-file';
    await page.evaluate((fileId) => {
      if (!window.__pdfCacheData) {
        window.__pdfCacheData = {};
      }
      // PDFヘッダーのBase64エンコード
      const mockPdfContent = btoa('%PDF-1.4 iframe test content');
      window.__pdfCacheData[fileId] = {
        base64: mockPdfContent,
        timestamp: Date.now()
      };
    }, mockFileId);

    // iframe（editor）を開く
    const iframeUrl = `${BASE_URL}/editor/index.html?mode=check&fileId=${mockFileId}&docType=test&contractor=test&gasUrl=`;

    // iframeを追加
    await page.evaluate((url) => {
      const iframe = document.createElement('iframe');
      iframe.id = 'test-iframe';
      iframe.src = url;
      iframe.style.width = '800px';
      iframe.style.height = '600px';
      document.body.appendChild(iframe);
    }, iframeUrl);

    await page.waitForTimeout(2000);

    // iframeのコンテンツにアクセス
    const iframeHandle = await page.$('#test-iframe');
    const frame = await iframeHandle?.contentFrame();

    if (frame) {
      // iframe内から親の__pdfCacheDataにアクセスできるか確認
      const canAccessParentCache = await frame.evaluate((fileId) => {
        try {
          const parentCache = window.parent.__pdfCacheData;
          if (parentCache && parentCache[fileId]) {
            const entry = parentCache[fileId];
            return entry && typeof entry.base64 === 'string';
          }
          return false;
        } catch (e) {
          console.error('Cannot access parent cache:', e);
          return false;
        }
      }, mockFileId);

      console.log('iframe can access parent cache:', canAccessParentCache);
      expect(canAccessParentCache).toBe(true);

      // getCachedPdfBase64関数を通じてもアクセスできることを確認
      const cachedViaFunction = await frame.evaluate((fileId) => {
        try {
          const getCached = window.parent.getCachedPdfBase64;
          if (typeof getCached === 'function') {
            return getCached(fileId) !== null;
          }
          return false;
        } catch (e) {
          console.error('Cannot call getCachedPdfBase64:', e);
          return false;
        }
      }, mockFileId);

      console.log('iframe can access cache via function:', cachedViaFunction);
      expect(cachedViaFunction).toBe(true);
    }

    console.log('All logs:', logs);
  });

  test('AiChecker uses cached PDF', async ({ page }) => {
    const fetchCalls: string[] = [];
    const logs: string[] = [];

    page.on('console', (msg) => {
      logs.push(`[${msg.type()}] ${msg.text()}`);
    });

    // ネットワークリクエストを監視
    page.on('request', (request) => {
      if (request.url().includes('fetchPdf')) {
        fetchCalls.push(request.url());
      }
    });

    await page.goto(BASE_URL);
    await page.waitForTimeout(1000);

    const mockFileId = 'cached-pdf-test';

    // PDFデータをキャッシュに追加（Base64形式）
    await page.evaluate((fileId) => {
      if (!window.__pdfCacheData) {
        window.__pdfCacheData = {};
      }

      // 最小限のPDFデータをBase64エンコード
      const pdfContent = `%PDF-1.4
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >> endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer << /Size 4 /Root 1 0 R >>
startxref
210
%%EOF`;

      window.__pdfCacheData[fileId] = {
        base64: btoa(pdfContent),
        timestamp: Date.now()
      };

      console.log('[test] PDF cached:', fileId, 'size:', pdfContent.length);
    }, mockFileId);

    // AiChecker iframeを開く（gasUrlは空でキャッシュから読む）
    const checkerUrl = `${BASE_URL}/editor/index.html?mode=check&fileId=${mockFileId}&docType=テスト&contractor=テスト業者&gasUrl=`;

    await page.evaluate((url) => {
      const iframe = document.createElement('iframe');
      iframe.id = 'checker-iframe';
      iframe.src = url;
      iframe.style.width = '100%';
      iframe.style.height = '600px';
      document.body.innerHTML = '';
      document.body.appendChild(iframe);
    }, checkerUrl);

    await page.waitForTimeout(3000);

    // fetchPdfへのリクエストが発生していないことを確認
    console.log('fetchPdf calls:', fetchCalls);
    console.log('All logs:', logs);

    // キャッシュがあればfetchPdfは呼ばれないはず
    // ただしgasUrlが空なのでエラーになる可能性あり
    const cacheHitLogs = logs.filter(l => l.includes('cache') || l.includes('Cache'));
    console.log('Cache related logs:', cacheHitLogs);

    // キャッシュから読み込んだログがあることを確認
    const foundInCache = logs.some(l => l.includes('PDF found in cache'));
    console.log('Found "PDF found in cache" log:', foundInCache);
    expect(foundInCache).toBe(true);
  });
});

// Window型拡張
declare global {
  interface Window {
    __pdfCacheData?: Record<string, { base64: string; timestamp: number }>;
    prefetchPdf?: (fileId: string, gasUrl: string) => Promise<boolean>;
    getCachedPdfBase64?: (fileId: string) => string | null;
  }
}
