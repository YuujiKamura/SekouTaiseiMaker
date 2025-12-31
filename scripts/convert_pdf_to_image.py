"""
PDFを画像に変換してdataフォルダに保存
OCR座標表示の背景用
"""

import json
import re
import sys
from pathlib import Path

from google.oauth2.credentials import Credentials
from google.auth.transport.requests import Request
import googleapiclient.discovery

# PyMuPDF for PDF to image
try:
    import fitz  # PyMuPDF
except ImportError:
    print("PyMuPDFが必要です: pip install pymupdf")
    sys.exit(1)

# 設定
PROJECT_ROOT = Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator")
TOKEN_PATH = PROJECT_ROOT / "gmail_token.json"
SEKOU_TAISEI_PATH = Path(r"H:\マイドライブ\〇市道 南千反畑町第１号線舗装補修工事\５施工体制")
OUTPUT_DIR = Path(r"C:\Users\yuuji\Sanyuu2Kouku\SekouTaiseiMaker\data\pdf_images")

SCOPES = [
    'https://www.googleapis.com/auth/drive',
    'https://www.googleapis.com/auth/spreadsheets',
    'https://mail.google.com/',
]


def get_drive_service():
    """Drive APIサービスを取得"""
    creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)
    if creds and creds.refresh_token:
        try:
            creds.refresh(Request())
        except Exception as e:
            print(f"トークンリフレッシュエラー: {e}")
    return googleapiclient.discovery.build('drive', 'v3', credentials=creds)


def extract_file_id(url):
    """URLからファイルIDを抽出"""
    patterns = [
        r'/file/d/([a-zA-Z0-9-_]+)',
        r'/d/([a-zA-Z0-9-_]+)',
        r'id=([a-zA-Z0-9-_]+)',
    ]
    for pattern in patterns:
        match = re.search(pattern, url)
        if match:
            return match.group(1)
    return None


def download_pdf(drive_service, file_id):
    """PDFをダウンロード"""
    request = drive_service.files().get_media(fileId=file_id)
    return request.execute()


def convert_pdf_to_png(pdf_content, output_path, dpi=150):
    """PDFをPNGに変換"""
    doc = fitz.open(stream=pdf_content, filetype="pdf")
    page = doc.load_page(0)

    # 指定DPIでレンダリング
    zoom = dpi / 72  # 72 DPIがデフォルト
    mat = fitz.Matrix(zoom, zoom)
    pix = page.get_pixmap(matrix=mat)

    # PNGとして保存
    pix.save(str(output_path))
    doc.close()

    return pix.width, pix.height


def main():
    # 出力ディレクトリ作成
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # sekoutaisei.jsonから対象書類を読み込み
    sekoutaisei_path = SEKOU_TAISEI_PATH / "sekoutaisei.json"

    with open(sekoutaisei_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    drive_service = get_drive_service()

    results = []

    for contractor in data.get('contractors', []):
        contractor_name = contractor.get('name', '')

        for doc_key in ['09_暴対法誓約書', '08_作業員名簿']:
            doc_info = contractor.get('docs', {}).get(doc_key, {})

            if not doc_info.get('status') or not doc_info.get('url'):
                continue

            url = doc_info['url']

            # スプレッドシートはスキップ
            if 'spreadsheets' in url:
                print(f"{contractor_name} {doc_key}: スプレッドシート形式のためスキップ")
                continue

            try:
                file_id = extract_file_id(url)
                if not file_id:
                    continue

                print(f"処理中: {contractor_name} - {doc_key}")

                # PDFダウンロード
                pdf_content = download_pdf(drive_service, file_id)

                # ファイル名を生成
                safe_name = f"{contractor_name}_{doc_key}".replace('/', '_').replace('\\', '_')
                output_path = OUTPUT_DIR / f"{safe_name}.png"

                # PNG変換
                width, height = convert_pdf_to_png(pdf_content, output_path)

                print(f"  保存: {output_path.name} ({width}x{height})")

                results.append({
                    'contractor': contractor_name,
                    'doc_type': doc_key,
                    'url': url,
                    'image_path': f"data/pdf_images/{safe_name}.png",
                    'image_size': {'width': width, 'height': height}
                })

            except Exception as e:
                print(f"  エラー: {e}")

    # 結果をJSONで保存
    result_path = OUTPUT_DIR / "pdf_images_index.json"
    with open(result_path, 'w', encoding='utf-8') as f:
        json.dump(results, f, ensure_ascii=False, indent=2)

    print(f"\n完了: {len(results)}件の画像を生成")
    print(f"インデックス: {result_path}")


if __name__ == '__main__':
    main()
