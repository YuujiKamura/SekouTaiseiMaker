/**
 * フィールド抽出サービス - AIで書類タイプを判定し動的にフィールドを抽出
 *
 * ## 変更履歴
 * - 2026-01-03: 正規化比較に変更（敬称・スペース・全角半角を無視）
 * - 2026-01-03: Excelシリアル値→日付変換、セル位置で作成日/提出日判別
 * - 2026-01-03: 動的フィールド生成に変更（固定4項目→AIが判断）
 * - 2026-01-03: 初期実装
 */

import { getApiKey } from './apiKey';

/**
 * Excelシリアル値を日付文字列に変換
 * Excelは1900年1月1日を1とするシリアル値（1900年がうるう年扱いのバグあり）
 */
function excelSerialToDate(serial: number): string {
  // Excelの基準日: 1899-12-30 (シリアル値0)
  // 1900年2月29日のバグを考慮して、60以上の場合は1日引く
  const adjustedSerial = serial > 60 ? serial - 1 : serial;
  const baseDate = new Date(1899, 11, 30); // 1899-12-30
  const resultDate = new Date(baseDate.getTime() + adjustedSerial * 24 * 60 * 60 * 1000);

  const year = resultDate.getFullYear();
  const month = resultDate.getMonth() + 1;
  const day = resultDate.getDate();

  // 和暦変換（令和）
  if (year >= 2019) {
    const reiwaYear = year - 2018;
    return `令和${reiwaYear}年${month}月${day}日`;
  } else if (year >= 1989) {
    const heiseiYear = year - 1988;
    return `平成${heiseiYear}年${month}月${day}日`;
  }

  return `${year}年${month}月${day}日`;
}

/**
 * 値がExcelの日付シリアル値かどうかを判定
 * 日付として妥当な範囲: 1990年〜2100年 → シリアル値 32874〜73050
 */
function isExcelDateSerial(value: string): boolean {
  const num = parseFloat(value);
  if (isNaN(num) || !Number.isInteger(num)) return false;
  // 1990年1月1日 = 32874, 2100年12月31日 = 73050
  return num >= 32874 && num <= 73050;
}

/**
 * 値を変換（Excelシリアル値なら日付に変換）
 */
function convertValue(value: string): string {
  if (isExcelDateSerial(value)) {
    return excelSerialToDate(parseInt(value, 10));
  }
  return value;
}

const GEMINI_MODEL = 'gemini-2.0-flash-exp';
const API_BASE = 'https://generativelanguage.googleapis.com/v1beta/models';

// 動的フィールド（AIが決定）
export interface DynamicField {
  label: string;        // フィールド名（例: "事業所名", "証紙購入額"）
  value: string | null; // 値
  cell: string | null;  // セル番地
  confidence: 'high' | 'medium' | 'low';
  validation?: 'ok' | 'warning' | 'error';  // 妥当性判定
  validationNote?: string;                   // 判定理由
}

// 抽出結果
export interface ExtractionResult {
  documentType: string;      // 書類タイプ（例: "作業員名簿", "建退共"）
  fields: DynamicField[];    // 抽出されたフィールド
}

// 工事情報（妥当性判定用）
export interface ProjectContext {
  contractor: string;           // 業者名
  projectName?: string;         // 工事名
  periodStart?: string;         // 工期開始日 (yyyy-MM-dd)
  periodEnd?: string;           // 工期終了日 (yyyy-MM-dd)
  siteRepresentative?: string;  // 現場代理人（元請け）= 所長名の正解
  today?: string;               // 今日の日付 (yyyy-MM-dd)
}

// 重要4項目（ソート用）
export const KEY_FIELDS = ['事業所名', '所長名', '作成日', '提出日'];

// 探索用プロンプト（工事情報プレースホルダー付き）
export function buildExtractionPrompt(context?: ProjectContext): string {
  const contextInfo = context
    ? `
【照合用の工事情報】
- 業者名: ${context.contractor}
${context.projectName ? `- 工事名: ${context.projectName}` : ''}
${context.siteRepresentative ? `- 現場代理人（所長名の正解）: ${context.siteRepresentative}` : ''}
${context.periodStart ? `- 工期開始日: ${context.periodStart}` : ''}
${context.periodEnd ? `- 工期終了日: ${context.periodEnd}` : ''}
${context.today ? `- 今日の日付: ${context.today}` : ''}
`
    : '';

  return `あなたは建設業書類のエキスパートです。
スプレッドシートデータを分析し、書類タイプを判定して重要なフィールドを抽出してください。

【データ形式】
セル番地:値 の形式で与えます。例: "A1:作業員名簿", "D3:工事名"
※日付はすでに和暦/西暦に変換済みです

【セル位置の解釈 - 重要】
作業員名簿の左上は「宛先」（提出先）である。報告する下請の名前ではない。

- 事業所名: 左上（A〜D列の1〜5行目）の「事業所の名称」欄
  = 元請が設置した工事事務所名、または工事名
  ※下請業者名ではない
- 所長名: 左上付近の「所長の氏名」欄 = 元請の現場代理人
- 作成日: 中央上部（E〜H列の1〜5行目）
- 提出日: 右上部（I列以降の1〜5行目）

作業員名簿のレイアウト:
- 左上: 宛先（事業所の名称=工事名or事務所名、所長の氏名=元請現場代理人）
- 中央上: 作成日
- 右上: 提出日
${contextInfo}
【タスク】
1. まず書類タイプを判定してください（作業員名簿、建退共、安全書類、施工体制台帳など）
2. 重要4項目を必ず抽出してください: 事業所名、所長名、作成日、提出日
3. その他の重要フィールドも抽出してください
4. 日付フィールドはセル位置から「作成日」「提出日」を正確に判別してください
5. 工事情報が与えられている場合、各フィールドの妥当性を判定してください

【妥当性判定基準】
このチェック機構の目的は「人間の手入力ミスを検出すること」。

- 事業所名: 照合用の工事名と一致なら ok、文字が違えば error
  ※スペース・全角半角の違いは無視
- 所長名: 人名部分だけを抽出して比較せよ
  肩書き（代表取締役、専務、現場代理人など）や敬称（殿、様など）は無視
  「代表取締役 池田　聡一郎　殿」と「池田聡一郎」は同一人物 → ok
  人名自体が違えば warning
- 作成日: 未来の日付でなければ ok（今日以前なら ok）
- 提出日: 未来の日付でなければ ok（今日以前なら ok）
  ※工期開始前に書類を準備・提出するのは普通
- 空欄や未検出は error

【回答形式】
以下のJSON形式で回答してください:
{
  "documentType": "書類タイプ",
  "fields": [
    { "label": "事業所名", "cell": "セル番地", "value": "値", "confidence": "high/medium/low", "validation": "ok/warning/error", "validationNote": "判定理由" },
    { "label": "所長名", ... },
    { "label": "作成日", ... },
    { "label": "提出日", ... },
    ...その他のフィールド
  ]
}

見つからない場合は cell と value を null、validation を "error" にしてください。
重要4項目（事業所名、所長名、作成日、提出日）を先頭に、最大10フィールドまで抽出してください。

【データ】
`;
}

/**
 * SheetJSのワークシートからセル番地付きデータを生成
 * Excelシリアル値は自動的に和暦/西暦に変換
 */
export function generateCellAddressedData(
  worksheet: Record<string, { v?: unknown; w?: string }>,
  maxRows: number = 20,
  maxCols: number = 26
): string {
  const lines: string[] = [];

  for (let row = 1; row <= maxRows; row++) {
    for (let col = 1; col <= maxCols; col++) {
      const colLetter = String.fromCharCode(64 + col); // A=65
      const cellAddress = `${colLetter}${row}`;
      const cell = worksheet[cellAddress];

      if (cell && cell.v !== undefined && cell.v !== null && cell.v !== '') {
        // フォーマット済み文字列があればそれを使用、なければ生値を変換
        let value = cell.w || String(cell.v);
        value = value.replace(/\n/g, ' ').trim();
        // SheetJSがフォーマットしていない場合のみシリアル値変換
        if (!cell.w) {
          value = convertValue(value);
        }
        if (value) {
          lines.push(`${cellAddress}:${value}`);
        }
      }
    }
  }

  return lines.join('\n');
}

/**
 * 2D配列からセル番地付きデータを生成（Google Spreadsheet用）
 * Excelシリアル値は自動的に和暦/西暦に変換
 */
export function generateCellAddressedDataFromArray(
  data: string[][],
  maxRows: number = 20,
  maxCols: number = 26
): string {
  const lines: string[] = [];

  for (let row = 0; row < Math.min(data.length, maxRows); row++) {
    const rowData = data[row] || [];
    for (let col = 0; col < Math.min(rowData.length, maxCols); col++) {
      let value = String(rowData[col] || '').replace(/\n/g, ' ').trim();
      if (value) {
        // Excelシリアル値を日付に変換
        value = convertValue(value);
        const colLetter = String.fromCharCode(65 + col);
        const cellAddress = `${colLetter}${row + 1}`;
        lines.push(`${cellAddress}:${value}`);
      }
    }
  }

  return lines.join('\n');
}

/**
 * AIを使って書類タイプを判定し、動的にフィールドを抽出
 * @param cellAddressedData セル番地付きデータ
 * @param context 工事情報（妥当性判定用）
 */
export async function extractFields(
  cellAddressedData: string,
  context?: ProjectContext
): Promise<ExtractionResult | null> {
  const apiKey = getApiKey();
  if (!apiKey) {
    console.error('[FieldExtractor] APIキーが設定されていません');
    return null;
  }

  const prompt = buildExtractionPrompt(context) + cellAddressedData;
  const url = `${API_BASE}/${GEMINI_MODEL}:generateContent?key=${apiKey}`;

  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        contents: [{ parts: [{ text: prompt }] }],
        generationConfig: {
          temperature: 0.1,
          maxOutputTokens: 1024,
        },
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error?.message || `API error: ${response.status}`);
    }

    const data = await response.json();
    const text = data.candidates?.[0]?.content?.parts?.[0]?.text;

    if (!text) {
      throw new Error('Empty response from Gemini');
    }

    // JSONを抽出してパース
    let jsonText = text;
    if (jsonText.includes('```')) {
      const match = jsonText.match(/```(?:json)?\s*([\s\S]*?)```/);
      if (match) {
        jsonText = match[1];
      }
    }

    const result = JSON.parse(jsonText.trim()) as ExtractionResult;

    // 重要4項目を先頭にソート
    result.fields = sortFieldsByPriority(result.fields);

    return result;

  } catch (e) {
    console.error('[FieldExtractor] Error:', e);
    return null;
  }
}

/**
 * フィールドを重要4項目優先でソート
 */
function sortFieldsByPriority(fields: DynamicField[]): DynamicField[] {
  return fields.sort((a, b) => {
    const aIndex = KEY_FIELDS.indexOf(a.label);
    const bIndex = KEY_FIELDS.indexOf(b.label);

    // 両方とも重要4項目の場合、定義順
    if (aIndex >= 0 && bIndex >= 0) {
      return aIndex - bIndex;
    }
    // aのみ重要4項目 → 前へ
    if (aIndex >= 0) return -1;
    // bのみ重要4項目 → 後ろへ
    if (bIndex >= 0) return 1;
    // どちらも重要4項目でない場合は順序維持
    return 0;
  });
}

/**
 * 日付フォーマットを整形
 */
export function formatDateValue(value: string | null): string | null {
  if (!value) return null;

  if (value.includes(' 00:00:00')) {
    return value.replace(' 00:00:00', '');
  }

  if (/^\d{4}-\d{2}-\d{2}/.test(value)) {
    return value.slice(0, 10);
  }

  return value;
}
