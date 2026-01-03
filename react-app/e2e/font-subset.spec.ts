import { test, expect } from '@playwright/test';

test.describe('Font Subset Tests', () => {
  test.beforeEach(async ({ page }) => {
    // コンソールログ・エラーを収集
    page.on('console', (msg) => {
      console.log(`[BROWSER ${msg.type().toUpperCase()}] ${msg.text()}`);
    });
    page.on('pageerror', (err) => {
      console.log(`[PAGE ERROR] ${err.message}`);
    });
    await page.goto('/');
  });

  // サブセット設定をテストするためのヘルパー
  async function testPdfSave(page: any, useSubset: boolean, testName: string) {
    const downloadPromise = page.waitForEvent('download');

    // テスト用PDFをアップロード
    const fileInput = page.locator('input[type="file"]');
    const pdfContent = await createTestPdf();

    await fileInput.setInputFiles({
      name: 'test.pdf',
      mimeType: 'application/pdf',
      buffer: pdfContent,
    });

    await expect(page.locator('.pdf-canvas')).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(2000);

    // サブセット設定を注入
    await page.evaluate((subset: boolean) => {
      (window as any).__TEST_SUBSET__ = subset;
    }, useSubset);

    // テキストを入力
    const textInput = page.locator('.text-input');
    await textInput.fill('テスト文字ABCあいう漢字');

    // キャンバスをクリックしてテキストを追加
    const canvas = page.locator('.overlay-canvas');
    await canvas.click({ position: { x: 100, y: 100 } });
    await page.waitForTimeout(500);

    // 保存
    const saveButton = page.locator('.save-btn');
    await saveButton.click();

    const download = await downloadPromise;
    const downloadedBuffer = await download.path();

    const fs = await import('fs');
    const pdfBytes = fs.readFileSync(downloadedBuffer!);

    // デスクトップに保存
    const desktopPath = `C:/Users/yuuji/Desktop/${testName}.pdf`;
    fs.writeFileSync(desktopPath, pdfBytes);

    console.log(`[${testName}] subset=${useSubset}, size=${pdfBytes.length} bytes (${(pdfBytes.length / 1024 / 1024).toFixed(2)} MB)`);

    return {
      size: pdfBytes.length,
      path: desktopPath,
    };
  }

  test('compare subset vs full embed', async ({ page }) => {
    // 現在の実装（subset無し）でテスト
    const result = await testPdfSave(page, false, 'full-embed');

    console.log(`Full embed: ${result.size} bytes`);
    console.log(`PDF saved to: ${result.path}`);

    // サイズが500KB以上（フォント埋め込み済み）
    expect(result.size).toBeGreaterThan(500000);
  });

  test('test with explicit subset options', async ({ page }) => {
    const downloadPromise = page.waitForEvent('download');

    const fileInput = page.locator('input[type="file"]');
    const pdfContent = await createTestPdf();

    await fileInput.setInputFiles({
      name: 'test.pdf',
      mimeType: 'application/pdf',
      buffer: pdfContent,
    });

    await expect(page.locator('.pdf-canvas')).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(2000);

    // 短いテキストでテスト（サブセットが効きやすい）
    const textInput = page.locator('.text-input');
    await textInput.fill('AB');

    const canvas = page.locator('.overlay-canvas');
    await canvas.click({ position: { x: 100, y: 100 } });
    await page.waitForTimeout(500);

    const saveButton = page.locator('.save-btn');
    await saveButton.click();

    const download = await downloadPromise;
    const downloadedBuffer = await download.path();

    const fs = await import('fs');
    const pdfBytes = fs.readFileSync(downloadedBuffer!);

    const desktopPath = 'C:/Users/yuuji/Desktop/short-text.pdf';
    fs.writeFileSync(desktopPath, pdfBytes);

    console.log(`Short text (AB only): ${pdfBytes.length} bytes (${(pdfBytes.length / 1024 / 1024).toFixed(2)} MB)`);
    console.log(`PDF saved to: ${desktopPath}`);

    expect(pdfBytes.length).toBeGreaterThan(0);
  });

  test('test japanese only text', async ({ page }) => {
    const downloadPromise = page.waitForEvent('download');

    const fileInput = page.locator('input[type="file"]');
    const pdfContent = await createTestPdf();

    await fileInput.setInputFiles({
      name: 'test.pdf',
      mimeType: 'application/pdf',
      buffer: pdfContent,
    });

    await expect(page.locator('.pdf-canvas')).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(2000);

    // 実際の会社名でテスト
    const textInput = page.locator('.text-input');
    await textInput.fill('有限会社　三雄建設');

    const canvas = page.locator('.overlay-canvas');
    await canvas.click({ position: { x: 100, y: 100 } });
    await page.waitForTimeout(500);

    const saveButton = page.locator('.save-btn');
    await saveButton.click();

    const download = await downloadPromise;
    const downloadedBuffer = await download.path();

    const fs = await import('fs');
    const pdfBytes = fs.readFileSync(downloadedBuffer!);

    const desktopPath = 'C:/Users/yuuji/Desktop/japanese-only.pdf';
    fs.writeFileSync(desktopPath, pdfBytes);

    console.log(`Japanese only (あ): ${pdfBytes.length} bytes (${(pdfBytes.length / 1024 / 1024).toFixed(2)} MB)`);
    console.log(`PDF saved to: ${desktopPath}`);

    expect(pdfBytes.length).toBeGreaterThan(0);
  });
});

async function createTestPdf(): Promise<Buffer> {
  const pdfContent = `%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>
endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer
<< /Size 4 /Root 1 0 R >>
startxref
210
%%EOF`;

  return Buffer.from(pdfContent, 'utf-8');
}
