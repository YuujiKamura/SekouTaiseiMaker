"""
GEMINI APIを使った書類チェッカー
"""
import os
import json
import base64
from pathlib import Path
from typing import Optional

import google.generativeai as genai
from googleapiclient.discovery import build

# APIキーのパス設定
# Windows環境: C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\credentials\gemini_api_key.txt
# Linux環境: 環境変数 GEMINI_API_KEY または カレントディレクトリの credentials/gemini_api_key.txt
API_KEY_PATHS = [
    Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\credentials\gemini_api_key.txt"),
    Path.home() / "credentials" / "gemini_api_key.txt",
    Path(__file__).parent.parent / "credentials" / "gemini_api_key.txt",
]


def get_api_key() -> str:
    """APIキーを取得"""
    # 環境変数から取得
    env_key = os.environ.get("GEMINI_API_KEY")
    if env_key:
        return env_key.strip()

    # ファイルから取得
    for path in API_KEY_PATHS:
        if path.exists():
            return path.read_text().strip()

    raise FileNotFoundError(
        "APIキーが見つかりません。環境変数 GEMINI_API_KEY を設定するか、"
        f"以下のいずれかにAPIキーファイルを配置してください: {API_KEY_PATHS}"
    )


def init_gemini():
    """GEMINI APIを初期化"""
    api_key = get_api_key()
    genai.configure(api_key=api_key)
    return genai.GenerativeModel('gemini-2.0-flash-exp')


def image_to_base64(image_path: Path) -> str:
    """画像をBase64エンコード"""
    with open(image_path, 'rb') as f:
        return base64.standard_b64encode(f.read()).decode('utf-8')


def get_mime_type(image_path: Path) -> str:
    """ファイル拡張子からMIMEタイプを取得"""
    ext = image_path.suffix.lower()
    mime_types = {
        '.png': 'image/png',
        '.jpg': 'image/jpeg',
        '.jpeg': 'image/jpeg',
        '.gif': 'image/gif',
        '.webp': 'image/webp',
        '.pdf': 'application/pdf',
    }
    return mime_types.get(ext, 'image/png')


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
    mime_type = get_mime_type(image_path)

    # GEMINI API呼び出し
    response = model.generate_content([
        prompt,
        {
            "mime_type": mime_type,
            "data": image_data
        }
    ])

    # レスポンスをパース
    try:
        # JSONブロックを抽出（```json ... ``` 形式に対応）
        text = response.text
        if "```json" in text:
            text = text.split("```json")[1].split("```")[0]
        elif "```" in text:
            text = text.split("```")[1].split("```")[0]
        result = json.loads(text.strip())
    except json.JSONDecodeError:
        result = {
            "status": "error",
            "summary": "レスポンスの解析に失敗",
            "items": [{"type": "info", "message": response.text}],
            "missing_fields": []
        }

    return result


def read_sheet_data(spreadsheet_id: str, sheet_name: Optional[str] = None) -> list:
    """
    Google Sheets APIでスプレッドシートデータを取得

    Args:
        spreadsheet_id: スプレッドシートID
        sheet_name: シート名（省略時は最初のシート）

    Returns:
        2次元リスト [[row1], [row2], ...]
    """
    api_key = os.environ.get("GOOGLE_API_KEY")
    if not api_key:
        raise ValueError("環境変数 GOOGLE_API_KEY を設定してください")

    service = build('sheets', 'v4', developerKey=api_key)

    # シート名が指定されていない場合は最初のシートを取得
    if not sheet_name:
        spreadsheet = service.spreadsheets().get(
            spreadsheetId=spreadsheet_id
        ).execute()
        sheet_name = spreadsheet['sheets'][0]['properties']['title']

    # データを取得
    data = service.spreadsheets().values().get(
        spreadsheetId=spreadsheet_id,
        range=f"'{sheet_name}'!A:Z"
    ).execute()

    return data.get('values', [])


def check_spreadsheet(
    spreadsheet_id: str,
    doc_type: str,
    contractor_name: str,
    sheet_name: Optional[str] = None
) -> dict:
    """
    スプレッドシートをGEMINIでチェック

    Args:
        spreadsheet_id: Google SpreadsheetのID
        doc_type: 書類タイプ
        contractor_name: 業者名
        sheet_name: シート名（省略時は最初のシート）

    Returns:
        check_pdf_imageと同じ形式
    """
    # Sheets APIでデータ取得
    sheet_data = read_sheet_data(spreadsheet_id, sheet_name)

    model = init_gemini()

    from document_prompts import get_spreadsheet_check_prompt
    prompt = get_spreadsheet_check_prompt(doc_type, contractor_name, sheet_data)

    response = model.generate_content(prompt)

    try:
        # JSONブロックを抽出（```json ... ``` 形式に対応）
        text = response.text
        if "```json" in text:
            text = text.split("```json")[1].split("```")[0]
        elif "```" in text:
            text = text.split("```")[1].split("```")[0]
        result = json.loads(text.strip())
    except json.JSONDecodeError:
        result = {
            "status": "error",
            "summary": "レスポンスの解析に失敗",
            "items": [{"type": "info", "message": response.text}],
            "missing_fields": []
        }

    return result


def check_multiple_pages(
    image_paths: list[Path],
    doc_type: str,
    contractor_name: str
) -> dict:
    """
    複数ページのPDF画像をまとめてチェック

    Args:
        image_paths: PNG画像のパスリスト
        doc_type: 書類タイプ
        contractor_name: 業者名

    Returns:
        check_pdf_imageと同じ形式（全ページを統合）
    """
    model = init_gemini()

    from document_prompts import get_check_prompt
    prompt = get_check_prompt(doc_type, contractor_name)

    # 全ページの画像を準備
    content = [prompt + f"\n\n※この書類は{len(image_paths)}ページあります。全ページを確認してください。"]

    for i, image_path in enumerate(image_paths, 1):
        image_data = image_to_base64(image_path)
        mime_type = get_mime_type(image_path)
        content.append({
            "mime_type": mime_type,
            "data": image_data
        })

    # GEMINI API呼び出し
    response = model.generate_content(content)

    try:
        text = response.text
        if "```json" in text:
            text = text.split("```json")[1].split("```")[0]
        elif "```" in text:
            text = text.split("```")[1].split("```")[0]
        result = json.loads(text.strip())
    except json.JSONDecodeError:
        result = {
            "status": "error",
            "summary": "レスポンスの解析に失敗",
            "items": [{"type": "info", "message": response.text}],
            "missing_fields": []
        }

    return result


if __name__ == '__main__':
    # テスト実行
    print("GEMINI Checker モジュール")
    print("使用例:")
    print("  from gemini_checker import check_pdf_image")
    print("  result = check_pdf_image(Path('image.png'), '暴対法誓約書', '業者名')")
