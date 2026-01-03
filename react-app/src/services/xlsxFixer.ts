/**
 * xlsxファイル修正サービス - xlsx-populateで書式を完全保持したまま修正
 *
 * ## 変更履歴
 * - 2026-01-03: xlsx-populateに移行（オートシェイプ含む書式完全保持）
 * - 2026-01-03: ExcelJS版
 * - 2026-01-03: 初期実装
 */

import XlsxPopulate from 'xlsx-populate';
import type { Workbook } from 'xlsx-populate';

/**
 * 修正情報
 */
export interface CellFix {
  sheetName: string;   // シート名
  cell: string;        // セル番地（例: "B3"）
  oldValue: string;    // 現在の値
  newValue: string;    // 修正後の値
  label: string;       // フィールド名（例: "事業所名"）
}

/**
 * xlsx-populateでワークブックを読み込む（書式完全保持）
 * @param data ArrayBuffer
 * @returns xlsx-populateワークブック
 */
export async function loadWorkbook(data: ArrayBuffer): Promise<Workbook> {
  return await XlsxPopulate.fromDataAsync(data);
}

/**
 * ワークブックのセルを修正する（書式を完全保持）
 * @param workbook 修正対象のワークブック
 * @param fixes 適用する修正リスト
 */
export function applyFixesToWorkbook(
  workbook: Workbook,
  fixes: CellFix[]
): void {
  for (const fix of fixes) {
    const sheet = workbook.sheet(fix.sheetName);
    if (!sheet) {
      console.warn(`[xlsxFixer] Sheet not found: ${fix.sheetName}`);
      continue;
    }

    // 値だけ更新（書式は自動的に保持される）
    sheet.cell(fix.cell).value(fix.newValue);

    console.log(`[xlsxFixer] Fixed ${fix.sheetName}!${fix.cell}: "${fix.oldValue}" → "${fix.newValue}"`);
  }
}

/**
 * 修正されたワークブックをBlobとしてエクスポート
 * @param workbook 修正済みワークブック
 * @returns Blob（xlsxファイル）
 */
export async function exportWorkbookAsBlob(workbook: Workbook): Promise<Blob> {
  // ブラウザ環境では 'blob' タイプを明示的に指定
  const blob = await workbook.outputAsync({ type: 'blob' }) as Blob;
  return blob;
}

/**
 * 修正されたxlsxファイルをダウンロード
 * @param workbook 修正済みワークブック
 * @param originalFileName 元のファイル名
 */
export async function downloadFixedXlsx(
  workbook: Workbook,
  originalFileName: string
): Promise<void> {
  const blob = await exportWorkbookAsBlob(workbook);
  const url = URL.createObjectURL(blob);

  // ファイル名生成（_修正済を追加）
  const baseName = originalFileName.replace(/\.xlsx?$/i, '');
  const timestamp = new Date().toISOString().slice(0, 10).replace(/-/g, '');
  const fixedFileName = `${baseName}_修正済_${timestamp}.xlsx`;

  const a = document.createElement('a');
  a.href = url;
  a.download = fixedFileName;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);

  console.log(`[xlsxFixer] Downloaded: ${fixedFileName}`);
}

/**
 * AI解析結果から修正候補を生成
 * @param fields AI解析で抽出されたフィールド
 * @param sheetName 対象シート名
 * @param projectContext 工事情報（期待値）
 * @returns 修正候補リスト
 */
export function generateFixCandidates(
  fields: Array<{
    label: string;
    value: string | null;
    cell: string | null;
    validation?: 'ok' | 'warning' | 'error';
    validationNote?: string;
  }>,
  sheetName: string,
  projectContext: {
    projectName?: string;
    siteRepresentative?: string;
    chiefEngineer?: string;
  }
): CellFix[] {
  const fixes: CellFix[] = [];

  for (const field of fields) {
    // validation が error または warning で、セル番地と値がある場合のみ
    if (
      (field.validation === 'error' || field.validation === 'warning') &&
      field.cell &&
      field.value
    ) {
      // 期待値を決定
      let expectedValue: string | null = null;

      if (field.label === '事業所名' && projectContext.projectName) {
        expectedValue = projectContext.projectName;
      } else if (field.label === '所長名' && projectContext.siteRepresentative) {
        expectedValue = projectContext.siteRepresentative;
      }

      // 期待値が決まっていれば修正候補に追加
      if (expectedValue) {
        fixes.push({
          sheetName,
          cell: field.cell,
          oldValue: field.value,
          newValue: expectedValue,
          label: field.label,
        });
      }
    }
  }

  return fixes;
}
