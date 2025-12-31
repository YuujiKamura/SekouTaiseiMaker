"""
書類タイプ別のチェックプロンプト
"""


class UnknownDocTypeError(Exception):
    """未知の書類タイプエラー"""
    pass


PROMPTS = {
    "暴対法誓約書": """
あなたは建設業の書類チェック専門家です。
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
{{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {{"type": "ok" | "warning" | "error", "message": "具体的な指摘"}}
    ],
    "missing_fields": [
        {{"field": "未記入項目名", "location": "位置の説明"}}
    ]
}}
""",

    "作業員名簿": """
あなたは建設業の書類チェック専門家です。
この「作業員名簿」を確認してください。

業者名: {contractor_name}

以下の項目をチェックしてください:
1. 作業員の氏名が記入されているか
2. 生年月日が記入されているか
3. 住所が記入されているか
4. 資格・免許欄に必要な資格が記載されているか
5. 健康保険・年金の加入状況が記載されているか
6. 雇入年月日が記入されているか

結果を以下のJSON形式で返してください:
{{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {{"type": "ok" | "warning" | "error", "message": "具体的な指摘"}}
    ],
    "missing_fields": [
        {{"field": "未記入項目名", "location": "位置の説明"}}
    ]
}}
""",

    "下請負契約書": """
あなたは建設業の書類チェック専門家です。
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
{{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {{"type": "ok" | "warning" | "error", "message": "具体的な指摘"}}
    ],
    "missing_fields": [
        {{"field": "未記入項目名", "location": "位置の説明"}}
    ]
}}
""",

    "施工体制台帳": """
あなたは建設業の書類チェック専門家です。
この「施工体制台帳」を確認してください。

業者名: {contractor_name}

以下の項目をチェックしてください:
1. 工事名・工事場所が記入されているか
2. 発注者情報が記入されているか
3. 元請負人の情報（許可番号含む）が記入されているか
4. 監理技術者・主任技術者の資格情報があるか
5. 下請負人の情報が正しく記載されているか
6. 工期が記入されているか

結果を以下のJSON形式で返してください:
{{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {{"type": "ok" | "warning" | "error", "message": "具体的な指摘"}}
    ],
    "missing_fields": [
        {{"field": "未記入項目名", "location": "位置の説明"}}
    ]
}}
""",

    "再下請負通知書": """
あなたは建設業の書類チェック専門家です。
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
{{
    "status": "ok" | "warning" | "error",
    "summary": "全体の評価（1文）",
    "items": [
        {{"type": "ok" | "warning" | "error", "message": "具体的な指摘"}}
    ],
    "missing_fields": [
        {{"field": "未記入項目名", "location": "位置の説明"}}
    ]
}}
""",
}


def get_check_prompt(doc_type: str, contractor_name: str) -> str:
    """
    書類タイプに応じたプロンプトを取得

    Args:
        doc_type: 書類タイプ
        contractor_name: 業者名

    Raises:
        UnknownDocTypeError: 未知の書類タイプの場合
    """
    if doc_type not in PROMPTS:
        raise UnknownDocTypeError(
            f"未知の書類タイプ: {doc_type}. "
            f"対応タイプ: {list(PROMPTS.keys())}"
        )
    template = PROMPTS[doc_type]
    return template.format(contractor_name=contractor_name)


def get_check_prompt_safe(doc_type: str, contractor_name: str, default: str = "暴対法誓約書") -> str:
    """
    書類タイプに応じたプロンプトを取得（フォールバックあり）

    明示的にフォールバックが必要な場合のみ使用

    Args:
        doc_type: 書類タイプ
        contractor_name: 業者名
        default: フォールバック先の書類タイプ
    """
    template = PROMPTS.get(doc_type, PROMPTS[default])
    return template.format(contractor_name=contractor_name)


def get_spreadsheet_check_prompt(doc_type: str, contractor_name: str, sheet_data: list) -> str:
    """スプレッドシート用プロンプト"""
    base_prompt = get_check_prompt(doc_type, contractor_name)

    data_text = "\n".join([
        "\t".join([str(cell) for cell in row])
        for row in sheet_data
    ])

    return f"""
{base_prompt}

以下がスプレッドシートのデータです:
```
{data_text}
```
"""
