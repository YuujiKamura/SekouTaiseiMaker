"""
GEMINI APIを使った書類チェッカー
"""
import os
import json
import base64
import re
import tempfile
import ipaddress
from pathlib import Path
from typing import Optional
from urllib.parse import urlparse
import socket

import requests

import google.generativeai as genai
from googleapiclient.discovery import build

from document_prompts import get_check_prompt, get_spreadsheet_check_prompt

# Geminiモデル名（モデル変更時はここを更新）
GEMINI_MODEL_NAME = 'gemini-2.0-flash-exp'

# APIキーのパス設定（Gemini API と Google Sheets API 共通）
# 環境変数 GOOGLE_API_KEY または以下のファイルから取得
API_KEY_PATHS = [
    Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\credentials\google_api_key.txt"),
    Path.home() / "credentials" / "google_api_key.txt",
    Path(__file__).parent.parent / "credentials" / "google_api_key.txt",
]

# モジュールレベルでモデルをキャッシュ（シングルトン）
_model: Optional[genai.GenerativeModel] = None


def get_api_key() -> str:
    """APIキーを取得（Gemini/Sheets共通）"""
    # 環境変数から取得
    env_key = os.environ.get("GOOGLE_API_KEY")
    if env_key:
        return env_key.strip()

    # ファイルから取得
    for path in API_KEY_PATHS:
        if path.exists():
            return path.read_text().strip()

    raise FileNotFoundError(
        "APIキーが見つかりません。環境変数 GOOGLE_API_KEY を設定するか、"
        f"以下のいずれかにAPIキーファイルを配置してください: {API_KEY_PATHS}"
    )


def get_gemini_model() -> genai.GenerativeModel:
    """Geminiモデルのシングルトンインスタンスを取得"""
    global _model
    if _model is None:
        api_key = get_api_key()
        genai.configure(api_key=api_key)
        _model = genai.GenerativeModel(GEMINI_MODEL_NAME)
    return _model


def init_gemini():
    """GEMINI APIを初期化（後方互換性のため残す）"""
    return get_gemini_model()


def _validate_file_path(file_path: Path) -> Path:
    """
    ファイルパスを検証し、パストラバーサル攻撃を防ぐ

    Args:
        file_path: 検証するファイルパス

    Returns:
        正規化されたファイルパス

    Raises:
        ValueError: 不正なパスの場合
    """
    # パスを正規化（シンボリックリンク解決、..の解決）
    resolved_path = file_path.resolve()

    # 許可されたベースディレクトリ
    allowed_bases = [
        Path(tempfile.gettempdir()).resolve(),  # システムのtempディレクトリ
        Path(__file__).parent.parent.resolve(),  # プロジェクトルート
        Path.home().resolve(),  # ユーザーホームディレクトリ
    ]

    # いずれかの許可されたベースディレクトリ内かチェック
    is_allowed = any(
        str(resolved_path).startswith(str(base))
        for base in allowed_bases
    )

    if not is_allowed:
        raise ValueError(f"アクセスが許可されていないパスです: {file_path}")

    # ファイルが存在するかチェック
    if not resolved_path.exists():
        raise ValueError(f"ファイルが存在しません: {file_path}")

    # ディレクトリではなくファイルかチェック
    if not resolved_path.is_file():
        raise ValueError(f"ファイルではありません: {file_path}")

    return resolved_path


def image_to_base64(image_path: Path) -> str:
    """画像をBase64エンコード"""
    # パストラバーサル対策
    validated_path = _validate_file_path(image_path)
    with open(validated_path, 'rb') as f:
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


def parse_gemini_response(response_text: str) -> dict:
    """
    Geminiレスポンスをパース

    Args:
        response_text: Geminiからのレスポンステキスト

    Returns:
        パース済みの結果辞書
    """
    # JSONブロックを抽出（```json ... ``` 形式に対応）
    text = response_text
    if "```json" in text:
        text = text.split("```json")[1].split("```")[0]
    elif "```" in text:
        text = text.split("```")[1].split("```")[0]

    try:
        return json.loads(text.strip())
    except json.JSONDecodeError:
        return {
            "status": "error",
            "summary": "レスポンスの解析に失敗",
            "items": [{"type": "info", "message": response_text[:500]}],
            "missing_fields": []
        }


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
    model = get_gemini_model()
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

    return parse_gemini_response(response.text)


def read_sheet_data(spreadsheet_id: str, sheet_name: Optional[str] = None) -> list:
    """
    Google Sheets APIでスプレッドシートデータを取得

    Args:
        spreadsheet_id: スプレッドシートID
        sheet_name: シート名（省略時は最初のシート）

    Returns:
        2次元リスト [[row1], [row2], ...]
    """
    api_key = get_api_key()
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

    model = get_gemini_model()
    prompt = get_spreadsheet_check_prompt(doc_type, contractor_name, sheet_data)

    response = model.generate_content(prompt)

    return parse_gemini_response(response.text)


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
    model = get_gemini_model()
    prompt = get_check_prompt(doc_type, contractor_name)

    # 全ページの画像を準備
    content = [prompt + f"\n\n※この書類は{len(image_paths)}ページあります。全ページを確認してください。"]

    for image_path in image_paths:
        image_data = image_to_base64(image_path)
        mime_type = get_mime_type(image_path)
        content.append({
            "mime_type": mime_type,
            "data": image_data
        })

    # GEMINI API呼び出し
    response = model.generate_content(content)

    return parse_gemini_response(response.text)


def check_image_base64(
    image_data: str,
    doc_type: str,
    contractor_name: str,
    mime_type: str = 'image/png'
) -> dict:
    """
    Base64エンコードされた画像をGEMINIでチェック

    Args:
        image_data: Base64エンコードされた画像データ
        doc_type: 書類タイプ
        contractor_name: 業者名
        mime_type: MIMEタイプ（デフォルト: image/png）

    Returns:
        check_pdf_imageと同じ形式
    """
    model = get_gemini_model()
    prompt = get_check_prompt(doc_type, contractor_name)

    response = model.generate_content([
        prompt,
        {
            "mime_type": mime_type,
            "data": image_data
        }
    ])

    return parse_gemini_response(response.text)


def _is_private_ip(hostname: str) -> bool:
    """
    ホスト名がプライベートIPアドレスを指しているか簡易チェック
    """
    try:
        ip = ipaddress.ip_address(hostname)
    except ValueError:
        # hostname is not a direct IP literal
        return False
    return ip.is_private or ip.is_loopback or ip.is_link_local


def _is_hostname_resolving_to_private_ip(hostname: str) -> bool:
    """
    ホスト名をDNS解決し、プライベートIP / ループバックアドレスに解決されるかをチェック
    """
    try:
        addrinfo_list = socket.getaddrinfo(hostname, None)
    except OSError:
        # 解決できないホスト名はここではプライベートとはみなさない
        return False

    for family, _, _, _, sockaddr in addrinfo_list:
        ip_str = None
        if family == socket.AF_INET:
            ip_str = sockaddr[0]
        elif family == socket.AF_INET6:
            ip_str = sockaddr[0]
        if ip_str is None:
            continue
        try:
            ip = ipaddress.ip_address(ip_str)
        except ValueError:
            continue
        if ip.is_private or ip.is_loopback or ip.is_link_local:
            return True
    return False


def _validate_external_url(url: str) -> str:
    """
    外部から指定されたURLを検証し、安全なURLのみ許可する

    現状では以下を許可:
      - https スキームのみ
      - ホスト名が明示的に許可されたドメインのいずれか
    """
    parsed = urlparse(url)

    if parsed.scheme != "https":
        raise ValueError("サポートされていないURLスキームです（httpsのみ許可）")

    if not parsed.hostname:
        raise ValueError("URLにホスト名が含まれていません")

    # SSRF対策: プライベートIP / ループバックアドレスは拒否（IPリテラルの場合）
    if _is_private_ip(parsed.hostname):
        raise ValueError("プライベートIPアドレスへのアクセスは許可されていません")

    allowed_hosts = {
        "drive.google.com",
        "docs.google.com",
    }

    if parsed.hostname not in allowed_hosts:
        raise ValueError(f"指定されたホスト({parsed.hostname})へのアクセスは許可されていません")

    # SSRF対策: 許可されたホスト名であっても、プライベートIPに解決される場合は拒否
    if _is_hostname_resolving_to_private_ip(parsed.hostname):
        raise ValueError("プライベートIPアドレスに解決されるホスト名へのアクセスは許可されていません")

    return url


def download_file_from_url(url: str) -> tuple[bytes, str]:
    """
    URLからファイルをダウンロード

    Args:
        url: ダウンロードURL (Google DriveやHTTP URL)

    Returns:
        (ファイルバイナリ, MIMEタイプ)
    """
    # 外部入力されたURLを検証 (SSRF対策)
    url = _validate_external_url(url)

    # Google Drive URLの場合、ダウンロード用URLに変換
    if "drive.google.com" in url:
        # /file/d/FILE_ID/view -> export/download形式に変換
        match = re.search(r'/d/([a-zA-Z0-9_-]+)', url)
        if match:
            file_id = match.group(1)
            url = f"https://drive.google.com/uc?export=download&id={file_id}"

    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
    }

    # SSRF対策: リダイレクト先のURLも逐次検証するため、手動でリダイレクトを追跡
    max_redirects = 5
    redirects_followed = 0
    current_url = url

    while True:
        response = requests.get(
            current_url,
            headers=headers,
            timeout=30,
            allow_redirects=False,
        )

        # リダイレクトでなければレスポンスをそのまま利用
        if response.is_redirect or response.is_permanent_redirect:
            if redirects_followed >= max_redirects:
                raise requests.exceptions.TooManyRedirects(
                    f"リダイレクト回数が上限({max_redirects})を超えました"
                )

            location = response.headers.get("Location")
            if not location:
                # Location ヘッダが無いリダイレクトは不正とみなす
                raise requests.exceptions.InvalidURL("リダイレクト先URLが不正です")

            # 相対URLの可能性があるため、絶対URLに解決
            next_url = requests.compat.urljoin(current_url, location)

            # リダイレクト先URLも再度検証
            next_url = _validate_external_url(next_url)

            current_url = next_url
            redirects_followed += 1
            continue

        # 200系など最終レスポンス
        response.raise_for_status()
        content_type = response.headers.get('Content-Type', 'application/pdf')
        return response.content, content_type


def check_document_from_url(
    url: str,
    doc_type: str,
    contractor_name: str
) -> dict:
    """
    URLから書類をダウンロードしてチェック

    Args:
        url: 書類のURL
        doc_type: 書類タイプ
        contractor_name: 業者名

    Returns:
        チェック結果 dict
    """
    # ファイルをダウンロード
    file_data, mime_type = download_file_from_url(url)

    # 一時ファイルに保存
    suffix = '.pdf' if 'pdf' in mime_type.lower() else '.png'
    with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as f:
        f.write(file_data)
        temp_path = Path(f.name)

    try:
        # 既存のチェック関数を呼び出し
        # gemini-2.0-flash-expはPDF対応なので直接チェック
        result = check_pdf_image(temp_path, doc_type, contractor_name)
        return result
    finally:
        # 一時ファイルを削除
        temp_path.unlink(missing_ok=True)


if __name__ == '__main__':
    # テスト実行
    print("GEMINI Checker モジュール")
    print("使用例:")
    print("  from gemini_checker import check_pdf_image")
    print("  result = check_pdf_image(Path('image.png'), '暴対法誓約書', '業者名')")
