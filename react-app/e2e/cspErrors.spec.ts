import { test, expect } from '@playwright/test';

const BASE_URL = 'http://localhost:8080';

test.describe('CSP Error Investigation', () => {
  test('Main page - check for CSP errors', async ({ page }) => {
    const cspErrors: string[] = [];

    page.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('frame-ancestors') || text.includes('Content Security Policy')) {
        cspErrors.push(text);
      }
    });

    page.on('pageerror', (err) => {
      if (err.message.includes('frame-ancestors')) {
        cspErrors.push(err.message);
      }
    });

    await page.goto(BASE_URL);
    await page.waitForTimeout(3000);

    console.log('=== Main page CSP errors ===');
    console.log('Count:', cspErrors.length);
    cspErrors.forEach((e, i) => console.log(`${i + 1}:`, e.substring(0, 200)));
  });

  test('React Editor page - check for CSP errors', async ({ page }) => {
    const cspErrors: string[] = [];

    page.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('frame-ancestors') || text.includes('Content Security Policy')) {
        cspErrors.push(text);
      }
    });

    // React editor directly
    await page.goto(`${BASE_URL}/editor/index.html`);
    await page.waitForTimeout(2000);

    console.log('=== React Editor CSP errors ===');
    console.log('Count:', cspErrors.length);
    cspErrors.forEach((e, i) => console.log(`${i + 1}:`, e.substring(0, 200)));
  });

  test('React AiChecker page - check for CSP errors', async ({ page }) => {
    const cspErrors: string[] = [];

    page.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('frame-ancestors') || text.includes('Content Security Policy')) {
        cspErrors.push(text);
      }
    });

    // React checker directly
    await page.goto(`${BASE_URL}/editor/index.html?mode=check&fileId=test&docType=test&contractor=test&gasUrl=`);
    await page.waitForTimeout(2000);

    console.log('=== React AiChecker CSP errors ===');
    console.log('Count:', cspErrors.length);
    cspErrors.forEach((e, i) => console.log(`${i + 1}:`, e.substring(0, 200)));
  });
});
