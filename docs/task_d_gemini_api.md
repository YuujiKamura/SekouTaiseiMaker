# Task D: GEMINI API統合

## 目的
GEMINI APIを使って、PDF/画像/スプレッドシートの内容をチェックする機能を実装する。

## 技術スタック
- Python 3.x
- google-generativeai (GEMINI API)
- google-auth (認証)
- PyMuPDF (PDF→画像変換)

## ファイル構成

```
SekouTaiseiMaker/
├── scripts/
│   ├── gemini_checker.py      # メインのチェッカー
│   ├── document_prompts.py    # 書類タイプ別プロンプト
│   └── run_gemini_check.py    # CLIエントリポイント
```

## 実装

### 1. gemini_checker.py

```python
"""
GEMINI APIを使った書類チェッカー
"""
import json
import base64
from pathlib import Path
from typing import Optional
import google.generativeai as genai

# APIキー設定
API_KEY_PATH = Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\credentials\gemini_api_key.txt")

def get_api_key() -> str:
    """APIキーを取得"""
    if API_KEY_PATH.exists():
        return API_KEY_PATH.read_text().strip()
    raise FileNotFoundError(f"APIキーファイルが見つかりません: {API_KEY_PATH}")

def init_gemini():
    """GEMINI APIを初期化"""
    api_key = get_api_key()
    genai.configure(api_key=api_key)
    return genai.GenerativeModel('gemini-2.0-flash-exp')

def image_to_base64(image_path: Path) -> str:
    """画像をBase64エンコード"""
    with open(image_path, 'rb') as f:
        return base64.standard_b64encode(f.read()).decode('utf-8')

def check_pdf_image(
    image_path: Path,
    doc_type: str,
    contractor_name: str
) -> dict:
    """
    PDF画像をGEMINIでチェック

    Args:
        image_path: PNG画像のパス
        doc_type: 書類タイプ（"暴対法誓約書", "作業員名簿" など）
        contractor_name: 業者名

    Returns:
        {
            "status": "ok" | "warning" | "error",
            "summary": "概要メッセージ",
            "items": [
                {"type": "ok", "message": "..."},
                {"type": "warning", "message": "..."},
                {"type": "error", "message": "..."},
            ],
            "missing_fields": [
                {"field": "日付", "location": "右上"},
                {"field": "署名", "location": "下部"},
            ]
        }
    """
    model = init_gemini()

    # 書類タイプに応じたプロンプトを取得
    from document_prompts import get_check_prompt
    prompt = get_check_prompt(doc_type, contractor_name)

    # 画像を読み込み
    image_data = image_to_base64(image_path)

    # GEMINI API呼び出し
    response = model.generate_content([
        prompt,
        {
            "mime_type": "image/png",
            "data": image_data
        }
    ])

    # レスポンスをパース
    try:
        result = json.loads(response.text)
    except json.JSONDecodeError:
        result = {
            "status": "error",
            "summary": "レスポンスの解析に失敗",
            "items": [{"type": "info", "message": response.text}],
            "missing_fields": []
        }

    return result


def check_spreadsheet(
    spreadsheet_id: str,
    doc_type: str,
    contractor_name: str
) -> dict:
    """
    スプレッドシートをGEMINIでチェック

    Args:
        spreadsheet_id: Google SpreadsheetのID
        doc_type: 書類タイプ
        contractor_name: 業者名

    Returns:
        check_pdf_imageと同じ形式
    """
    # Sheets APIでデータ取得
    from read_spreadsheet import read_sheet_data
    sheet_data = read_sheet_data(spreadsheet_id)

    model = init_gemini()

    from document_prompts import get_spreadsheet_check_prompt
    prompt = get_spreadsheet_check_prompt(doc_type, contractor_name, sheet_data)

    response = model.generate_content(prompt)

    try:
        result = json.loads(response.text)
    except json.JSONDecodeError:
        result = {
            "status": "error",
            "summary": "レスポンスの解析に失敗",
            "items": [{"type": "info", "message": response.text}],
            "missing_fields": []
        }

    return result
```

### 2. document_prompts.py

```python
"""
書類タイプ別のチェックプロンプト
"""

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
}

def get_check_prompt(doc_type: str, contractor_name: str) -> str:
    """書類タイプに応じたプロンプトを取得"""
    template = PROMPTS.get(doc_type, PROMPTS.get("暴対法誓約書"))
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
```

### 3. run_gemini_check.py

```python
"""
CLIからGEMINIチェックを実行
"""
import argparse
import json
import sys
from pathlib import Path

from gemini_checker import check_pdf_image, check_spreadsheet


def main():
    parser = argparse.ArgumentParser(description='GEMINI書類チェッカー')
    parser.add_argument('--type', required=True, choices=['pdf', 'spreadsheet'])
    parser.add_argument('--path', help='PDF画像のパス')
    parser.add_argument('--spreadsheet-id', help='スプレッドシートID')
    parser.add_argument('--doc-type', required=True, help='書類タイプ')
    parser.add_argument('--contractor', required=True, help='業者名')
    parser.add_argument('--output', help='出力JSONファイル')

    args = parser.parse_args()

    if args.type == 'pdf':
        if not args.path:
            print("--path が必要です", file=sys.stderr)
            sys.exit(1)
        result = check_pdf_image(
            Path(args.path),
            args.doc_type,
            args.contractor
        )
    else:
        if not args.spreadsheet_id:
            print("--spreadsheet-id が必要です", file=sys.stderr)
            sys.exit(1)
        result = check_spreadsheet(
            args.spreadsheet_id,
            args.doc_type,
            args.contractor
        )

    output = json.dumps(result, ensure_ascii=False, indent=2)

    if args.output:
        Path(args.output).write_text(output, encoding='utf-8')
        print(f"結果を保存: {args.output}")
    else:
        print(output)


if __name__ == '__main__':
    main()
```

## 使用例

```bash
# PDF画像をチェック
python run_gemini_check.py \
    --type pdf \
    --path "data/pdf_images/下請A_09_暴対法誓約書.png" \
    --doc-type "暴対法誓約書" \
    --contractor "下請A"

# スプレッドシートをチェック
python run_gemini_check.py \
    --type spreadsheet \
    --spreadsheet-id "1Tm6alT13Jno_Fcq0Ml5OvPh9RqKNkIdXxhoVRqK--a8" \
    --doc-type "作業員名簿" \
    --contractor "下請B"
```

## Rust側からの呼び出し

WASMからは直接Pythonを呼べないので、以下の方法を検討:

1. **ローカルHTTPサーバー**: Pythonでシンプルなチェックサーバーを立てる
2. **事前実行**: ビルド時にすべての書類をチェックしてJSONに保存
3. **Cloudflare Workers**: Python→JSに移植してエッジで実行

### 推奨: ローカルHTTPサーバー

```python
# gemini_server.py
from flask import Flask, request, jsonify
from gemini_checker import check_pdf_image, check_spreadsheet

app = Flask(__name__)

@app.route('/check/pdf', methods=['POST'])
def check_pdf():
    data = request.json
    result = check_pdf_image(
        Path(data['image_path']),
        data['doc_type'],
        data['contractor']
    )
    return jsonify(result)

@app.route('/check/spreadsheet', methods=['POST'])
def check_ss():
    data = request.json
    result = check_spreadsheet(
        data['spreadsheet_id'],
        data['doc_type'],
        data['contractor']
    )
    return jsonify(result)

if __name__ == '__main__':
    app.run(port=5000)
```

## 出力ファイル
- `scripts/gemini_checker.py`
- `scripts/document_prompts.py`
- `scripts/run_gemini_check.py`
- `scripts/gemini_server.py` (オプション)

## 注意事項
- APIキーは `credentials/gemini_api_key.txt` に保存
- GEMINI 2.0 Flash を使用（高速・低コスト）
- レート制限に注意（1分あたりのリクエスト数）
