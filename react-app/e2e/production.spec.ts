import { test, expect } from '@playwright/test';

const PROD_URL = 'https://yuujikamura.github.io/SekouTaiseiMaker/editor/';

test.describe('Production PDF Editor', () => {
  test('loads fonts correctly on GitHub Pages', async ({ page }) => {
    const fontResponses: { url: string; status: number }[] = [];

    page.on('response', (response) => {
      if (response.url().includes('/fonts/') && response.url().includes('Noto')) {
        fontResponses.push({
          url: response.url(),
          status: response.status(),
        });
      }
    });

    await page.goto(PROD_URL);
    await page.waitForTimeout(3000);

    console.log('Font responses:', fontResponses);

    // フォントリクエストが成功していることを確認
    expect(fontResponses.length).toBeGreaterThan(0);

    // 各フォントが200 OKで読み込まれていることを確認
    for (const { url, status } of fontResponses) {
      console.log(`Font: ${url} -> ${status}`);
      expect(status).toBe(200);
    }
  });

  test('save works without font error', async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    await page.goto(PROD_URL);
    await page.waitForTimeout(2000);

    // テスト用PDFをアップロード
    const fileInput = page.locator('input[type="file"]');
    const pdfContent = createTestPdf();

    await fileInput.setInputFiles({
      name: 'test.pdf',
      mimeType: 'application/pdf',
      buffer: pdfContent,
    });

    await expect(page.locator('.pdf-canvas')).toBeVisible({ timeout: 15000 });

    // テキストを入力して追加
    const textInput = page.locator('.text-input');
    await textInput.fill('テスト');

    const canvas = page.locator('.overlay-canvas');
    await canvas.click({ position: { x: 100, y: 100 } });

    await page.waitForTimeout(500);

    // 保存ボタンをクリック
    const saveButton = page.locator('.save-btn');
    await saveButton.click();

    await page.waitForTimeout(5000);

    // フォントエラーが発生していないことを確認
    const fontErrors = consoleErrors.filter(e =>
      e.includes('Unknown font format') ||
      e.includes('Font load error')
    );

    console.log('All console errors:', consoleErrors);
    console.log('Font errors:', fontErrors);

    expect(fontErrors).toHaveLength(0);
  });
});

function createTestPdf(): Buffer {
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
