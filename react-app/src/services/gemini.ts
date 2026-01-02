/**
 * Gemini API サービス - ブラウザから直接呼び出し
 *
 * ## 変更履歴
 * - 2026-01-02: 入場年月日等をチェック対象外として明示（全プロンプト）
 * - 2026-01-02: docTypeを書類タイプとして明示表示
 */

import { getApiKey } from './apiKey';

const GEMINI_MODEL = 'gemini-2.0-flash-exp';
const API_BASE = 'https://generativelanguage.googleapis.com/v1beta/models';

export interface CheckResult {
  status: 'ok' | 'warning' | 'error';
  summary: string;
  items: Array<{ type: 'ok' | 'warning' | 'error'; message: string }>;
  missing_fields: Array<{ field: string; location: string }>;
}

const PROMPTS: Record<string, string> = {
  "暴対法誓約書": `あなたは建設業の書類チェック専門家です。
この「暴力団排除に関する誓約書」を確認してください。

業者名: {contractor_name}

以下の項目をチェックしてください:
1. 日付が記入されているか（「令和○年○月○日」の形式）
2. 宛先（発注者名）が正しく記入されているか
3. 誓約者の住所が記入されているか
4. 誓約者の氏名が記入されているか
5. 代表者の役職と氏名が記入されているか
6. 印鑑が押されているか（角印・丸印）

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`,

  "作業員名簿": `あなたは建設業の書類チェック専門家です。
この書類は「作業員名簿」です。

業者名: {contractor_name}

【チェック項目】
1. 作業員の氏名が記入されているか
2. 生年月日が記入されているか
3. 住所が記入されているか
4. 資格・免許欄に必要な資格が記載されているか
5. 健康保険・年金の加入状況が記載されているか
6. 雇入年月日が記入されているか

【重要：チェック対象外の項目】
- 入場年月日 → チェックしないでください。空白で正常です（作業日当日に記入するため）
- 退場年月日 → チェックしないでください
- 受入教育実施年月日 → チェックしないでください

上記3項目が空白・未記入でも「未記入」として報告しないでください。

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`,

  "下請負契約書": `あなたは建設業の書類チェック専門家です。
この「下請負契約書」を確認してください。

業者名: {contractor_name}

以下の項目をチェックしてください:
1. 契約日が記入されているか
2. 工事名・工事場所が記入されているか
3. 工期（着工日・完成日）が記入されているか
4. 請負代金が記入されているか
5. 元請・下請双方の記名押印があるか
6. 収入印紙が貼付されているか

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`,

  "施工体制台帳": `あなたは建設業の書類チェック専門家です。
この書類は「施工体制台帳」です。

業者名: {contractor_name}

【チェック項目】
1. 工事名・工事場所が記入されているか
2. 発注者情報が記入されているか
3. 元請負人の情報（許可番号含む）が記入されているか
4. 監理技術者・主任技術者の資格情報があるか
5. 下請負人の情報が正しく記載されているか
6. 工期が記入されているか

【重要：チェック対象外の項目】
- 入場年月日 → チェックしないでください（作業日当日に記入）
- 退場年月日 → チェックしないでください
- 受入教育実施年月日 → チェックしないでください

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`,

  "再下請負通知書": `あなたは建設業の書類チェック専門家です。
この「再下請負通知書」を確認してください。

業者名: {contractor_name}

以下の項目をチェックしてください:
1. 通知日が記入されているか
2. 元請負人への宛先が正しいか
3. 再下請負人の情報（社名・住所・許可番号）が記入されているか
4. 工事内容・工期が記入されているか
5. 契約金額が記入されているか
6. 通知者の記名押印があるか

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`,

};

// 労働保険番号確認用プロンプト
const ROUDOU_HOKEN_PROMPT = `あなたは建設業の書類チェック専門家です。
この書類が「労働保険番号」の証明として有効かどうかを確認してください。

業者名: {contractor_name}

【重要】労働保険番号確認書類として有効な条件:
1. 労働保険番号が記載されていること
2. 口座振替日（引落日）が記載されていること
3. 口座名義が当該法人（業者名）のものであること

※金額がマスキング・黒塗りされていても問題ありません
※口座番号がマスキング・黒塗りされていても問題ありません
※重要なのは「労働保険番号」「口座振替日」「口座名義が法人名と一致」の3点です

チェック項目:
1. 労働保険番号が確認できるか
2. 口座振替日（引落日）が確認できるか
3. 口座名義が業者名（{contractor_name}）と一致または関連しているか
4. 書類の形式が労働保険関連の公的書類として妥当か

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`;

// 法定外労災加入証明用プロンプト
const HOUTEI_GAI_ROUSAI_PROMPT = `あなたは建設業の書類チェック専門家です。
この書類が「法定外労災」の加入証明として有効かどうかを確認してください。

業者名: {contractor_name}

【重要】法定外労災加入証明として有効な条件:
1. 保険会社名・保険種別が確認できること
2. 被保険者（加入者）が業者名と一致すること
3. 保険期間（始期・終期）が明記されていること

【保険期間に関する重要な判定基準】
- 保険期間が工事期間をカバーしている場合 → OK
- 保険期間が工事期間内に終了する場合 → WARNING（要更新確認）
- 保険が既に失効している場合 → ERROR

※現在の日付と保険終期を比較し、近い将来（3ヶ月以内など）に期限切れになる場合は警告してください
※保険証券番号等がマスキングされていても問題ありません

チェック項目:
1. 保険会社名が確認できるか
2. 被保険者名が業者名（{contractor_name}）と一致するか
3. 保険期間の始期・終期が確認できるか
4. 保険期間が現在有効か（終期が過去でないか）
5. 保険期間が近い将来に終了しないか（警告対象）

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）- 保険期間の終期が近い場合は必ず言及すること",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`;

// 在籍証明系プロンプト（「在籍」を含む書類タイプ用）
const ZAISEKI_PROMPT = `あなたは建設業の書類チェック専門家です。
この書類が「{doc_type}」の証明として有効かどうかを確認してください。

業者名: {contractor_name}

【重要】在籍証明として有効な書類（以下のいずれかであればOK）:
- 健康保険証（本人の氏名と事業所名/会社名が記載されていればOK）
- 健康保険被保険者証
- 在籍証明書
- 雇用証明書
- 社員証（顔写真付き）
- その他、当該人物がその会社に所属していることを証明できる公的書類

チェック項目:
1. 本人の氏名が確認できるか
2. 事業所名・会社名が確認できるか（健康保険証の場合は保険者名または事業所名）
3. 上記の有効な書類のいずれかに該当するか

※健康保険証・健康保険被保険者証は、本人氏名と会社名（事業所名）が確認できれば在籍証明として有効です。
※個人情報（保険者番号、被保険者番号等）がマスキングされていても問題ありません。
※書類の種類が「{doc_type}」と完全一致しなくても、在籍を証明できる書類であればOKです。

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`;

// 汎用プロンプト（未知の書類タイプ用）
const GENERIC_PROMPT = `あなたは建設業の書類チェック専門家です。

書類タイプ: {doc_type}
業者名: {contractor_name}

【チェック項目】
1. 日付が記入されているか
2. 必要な記名・押印があるか
3. 必須項目が埋まっているか
4. 書類の形式が正しいか

【重要：チェック対象外の項目】
- 入場年月日 → チェックしないでください（作業日当日に記入するため）
- 退場年月日 → チェックしないでください
- 受入教育実施年月日 → チェックしないでください

上記項目が空白でも「未記入」として報告しないでください。

結果を以下のJSON形式で返してください:
{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {"type": "ok" | "warning" | "error", "message": "具体的な指摘"}
    ],
    "missing_fields": [
        {"field": "未記入項目名", "location": "位置の説明"}
    ]
}`;

function getPrompt(docType: string, contractorName: string): string {
  let template: string;
  if (PROMPTS[docType]) {
    template = PROMPTS[docType];
  } else if (docType.includes('法定外労災') || docType.includes('法廷外労災')) {
    // 「法定外労災」を含む書類は法定外労災加入証明用プロンプトを使用
    template = HOUTEI_GAI_ROUSAI_PROMPT;
  } else if (docType.includes('労働保険')) {
    // 「労働保険」を含む書類は労働保険番号確認用プロンプトを使用
    template = ROUDOU_HOKEN_PROMPT;
  } else if (docType.includes('在籍')) {
    // 「在籍」を含む書類は在籍証明用プロンプトを使用
    template = ZAISEKI_PROMPT.replace(/{doc_type}/g, docType);
  } else {
    template = GENERIC_PROMPT.replace('{doc_type}', docType);
  }
  return template.replace(/{contractor_name}/g, contractorName);
}

function parseResponse(text: string): CheckResult {
  let jsonText = text;

  // ```json ... ``` を除去
  if (jsonText.includes('```')) {
    const match = jsonText.match(/```(?:json)?\s*([\s\S]*?)```/);
    if (match) {
      jsonText = match[1];
    }
  }

  try {
    return JSON.parse(jsonText.trim());
  } catch {
    return {
      status: 'error',
      summary: 'レスポンスの解析に失敗',
      items: [{ type: 'error', message: text.slice(0, 500) }],
      missing_fields: [],
    };
  }
}

export async function checkDocumentImage(
  imageBase64: string,
  mimeType: string,
  docType: string,
  contractorName: string
): Promise<CheckResult> {
  const apiKey = getApiKey();
  if (!apiKey) {
    return {
      status: 'error',
      summary: 'APIキーが設定されていません',
      items: [{ type: 'error', message: 'メニュー → APIキー設定 から設定してください' }],
      missing_fields: [],
    };
  }

  const prompt = getPrompt(docType, contractorName);
  const url = `${API_BASE}/${GEMINI_MODEL}:generateContent?key=${apiKey}`;

  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        contents: [{
          parts: [
            { text: prompt },
            { inline_data: { mime_type: mimeType, data: imageBase64 } },
          ],
        }],
        generationConfig: {
          temperature: 0.1,
          maxOutputTokens: 2048,
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

    return parseResponse(text);
  } catch (e) {
    return {
      status: 'error',
      summary: 'API呼び出しエラー',
      items: [{ type: 'error', message: e instanceof Error ? e.message : String(e) }],
      missing_fields: [],
    };
  }
}

export async function checkSpreadsheet(
  sheetData: string[][],
  docType: string,
  contractorName: string
): Promise<CheckResult> {
  const apiKey = getApiKey();
  if (!apiKey) {
    return {
      status: 'error',
      summary: 'APIキーが設定されていません',
      items: [{ type: 'error', message: 'メニュー → APIキー設定 から設定してください' }],
      missing_fields: [],
    };
  }

  const basePrompt = getPrompt(docType, contractorName);
  const dataText = sheetData.map(row => row.join('\t')).join('\n');
  const prompt = `${basePrompt}\n\n以下がスプレッドシートのデータです:\n\`\`\`\n${dataText}\n\`\`\``;

  const url = `${API_BASE}/${GEMINI_MODEL}:generateContent?key=${apiKey}`;

  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        contents: [{
          parts: [{ text: prompt }],
        }],
        generationConfig: {
          temperature: 0.1,
          maxOutputTokens: 2048,
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

    return parseResponse(text);
  } catch (e) {
    return {
      status: 'error',
      summary: 'API呼び出しエラー',
      items: [{ type: 'error', message: e instanceof Error ? e.message : String(e) }],
      missing_fields: [],
    };
  }
}

export const DOC_TYPES = Object.keys(PROMPTS);
