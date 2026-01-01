import { test, expect } from '@playwright/test';

test.describe('PDF Editor', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('loads fonts correctly', async ({ page }) => {
    // フォントファイルが正しく読み込まれることを確認
    const fontResponses: { url: string; status: number }[] = [];

    page.on('response', (response) => {
      if (response.url().includes('/fonts/') && response.url().includes('Noto')) {
        fontResponses.push({
          url: response.url(),
          status: response.status(),
        });
      }
    });

    await page.reload();
    await page.waitForTimeout(2000);

    // フォントリクエストが成功していることを確認
    expect(fontResponses.length).toBeGreaterThan(0);

    // 各フォントが200 OKで読み込まれていることを確認
    for (const { url, status } of fontResponses) {
      expect(url).toContain('/fonts/Noto');
      expect(status).toBe(200);
    }

    // 明朝体とゴシック体の両方が読み込まれていること
    const urls = fontResponses.map(r => r.url);
    expect(urls.some(u => u.includes('Serif'))).toBe(true);
    expect(urls.some(u => u.includes('Sans'))).toBe(true);
  });

  test('shows file upload hint when no PDF loaded', async ({ page }) => {
    const hint = page.locator('.file-upload-hint');
    await expect(hint).toBeVisible();
    await expect(hint).toContainText('PDF');
  });

  test('can load a PDF file', async ({ page }) => {
    // テスト用PDFをアップロード
    const fileInput = page.locator('input[type="file"]');

    // テスト用の簡単なPDFを作成
    const pdfContent = await createTestPdf();

    await fileInput.setInputFiles({
      name: 'test.pdf',
      mimeType: 'application/pdf',
      buffer: pdfContent,
    });

    // PDFがロードされることを確認
    await expect(page.locator('.pdf-canvas')).toBeVisible({ timeout: 10000 });
  });

  test('save button works without font error', async ({ page }) => {
    // コンソールエラーを監視
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    // テスト用PDFをアップロード
    const fileInput = page.locator('input[type="file"]');
    const pdfContent = await createTestPdf();

    await fileInput.setInputFiles({
      name: 'test.pdf',
      mimeType: 'application/pdf',
      buffer: pdfContent,
    });

    await expect(page.locator('.pdf-canvas')).toBeVisible({ timeout: 10000 });

    // テキストを入力
    const textInput = page.locator('.text-input');
    await textInput.fill('テスト文字列');

    // キャンバスをクリックしてテキストを追加
    const canvas = page.locator('.overlay-canvas');
    await canvas.click({ position: { x: 100, y: 100 } });

    // 少し待ってアノテーションが追加されることを確認
    await page.waitForTimeout(500);

    // 保存ボタンをクリック
    const saveButton = page.locator('.save-btn');
    await saveButton.click();

    // 保存処理を待つ
    await page.waitForTimeout(3000);

    // フォントエラーが発生していないことを確認
    const fontErrors = consoleErrors.filter(e =>
      e.includes('Unknown font format') ||
      e.includes('Font load error')
    );
    expect(fontErrors).toHaveLength(0);
  });
});

// テスト用の簡単なPDFを生成
async function createTestPdf(): Promise<Buffer> {
  // 最小限のPDF構造
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
