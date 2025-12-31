"""
暴対法誓約書・作業員名簿の空欄フィールドの座標を取得
Document AI OCRを使用して「殿」「御中」「令和　年　月　日」の位置を特定
"""

import json
import re
import base64
from pathlib import Path
from google.cloud import documentai_v1 as documentai
from google.oauth2 import service_account
from google.oauth2.credentials import Credentials
import googleapiclient.discovery

# 設定
PROJECT_ROOT = Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator")
DOCUMENTAI_CONFIG = PROJECT_ROOT / "ocr_tools" / "credential" / "documentai_config.json"
CREDENTIALS_PATH = PROJECT_ROOT / "credentials" / "visionapi-437405-734d18d13418.json"
TOKEN_PATH = PROJECT_ROOT / "gmail_token.json"

DRIVE_SCOPES = [
    'https://www.googleapis.com/auth/drive',
    'https://www.googleapis.com/auth/spreadsheets',
    'https://mail.google.com/',
]

def load_documentai_config():
    """Document AI設定を読み込む"""
    with open(DOCUMENTAI_CONFIG, 'r', encoding='utf-8') as f:
        return json.load(f)

def get_documentai_client(config):
    """Document AIクライアントを取得"""
    cred_path = PROJECT_ROOT / config['credential_path']
    credentials = service_account.Credentials.from_service_account_file(
        str(cred_path),
        scopes=['https://www.googleapis.com/auth/cloud-platform']
    )
    return documentai.DocumentProcessorServiceClient(credentials=credentials)

def get_drive_service():
    """Drive APIサービスを取得"""
    from google.auth.transport.requests import Request
    creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), DRIVE_SCOPES)
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

def process_pdf_with_documentai(client, config, pdf_content):
    """Document AIでPDFを処理して座標付きテキストを取得"""
    processor_name = f"projects/{config['project_id']}/locations/{config['location']}/processors/{config['processor_id']}"

    raw_document = documentai.RawDocument(
        content=pdf_content,
        mime_type="application/pdf"
    )

    request = documentai.ProcessRequest(
        name=processor_name,
        raw_document=raw_document
    )

    result = client.process_document(request=request)
    return result.document

def find_blank_field_coordinates(document):
    """空欄フィールドの座標を検出"""
    results = {
        'destination': None,  # 提出先（殿/御中の位置）
        'date': None,         # 日付（令和　年　月　日の位置）
        'all_tokens': []
    }

    for page_num, page in enumerate(document.pages):
        page_height = page.dimension.height
        page_width = page.dimension.width

        for token in page.tokens:
            # テキストを取得
            token_text = ""
            for segment in token.layout.text_anchor.text_segments:
                start = segment.start_index if segment.start_index else 0
                end = segment.end_index
                token_text += document.text[start:end]

            token_text = token_text.strip()
            if not token_text:
                continue

            # バウンディングボックスの座標（正規化済み 0-1）
            vertices = token.layout.bounding_poly.normalized_vertices
            if not vertices:
                continue

            coords = {
                'text': token_text,
                'page': page_num + 1,
                'normalized': {
                    'x': vertices[0].x,
                    'y': vertices[0].y,
                    'width': vertices[2].x - vertices[0].x,
                    'height': vertices[2].y - vertices[0].y,
                },
                'pixels': {
                    'x': int(vertices[0].x * page_width),
                    'y': int(vertices[0].y * page_height),
                    'width': int((vertices[2].x - vertices[0].x) * page_width),
                    'height': int((vertices[2].y - vertices[0].y) * page_height),
                },
                'page_size': {
                    'width': page_width,
                    'height': page_height
                }
            }

            results['all_tokens'].append(coords)

            # 提出先フィールド検出（「殿」「御」「中」「様」を含む）
            # Document AIは「御中」を「御」「中」に分割することがある
            if token_text in ['殿', '御', '中', '様', '御中'] or token_text.endswith('殿') or token_text.endswith('御中') or token_text.endswith('様'):
                # 「御」が見つかった場合は、その左側が記入位置
                if token_text in ['御', '殿', '様'] or token_text.endswith('殿') or token_text.endswith('様'):
                    results['destination'] = {
                        'marker': token_text,
                        'marker_coords': coords,
                        'fill_position': {
                            'x': max(0, coords['normalized']['x'] - 0.20),  # マーカーの左側
                            'y': coords['normalized']['y'],
                            'suggested_width': 0.18,
                        }
                    }

            # 日付フィールド検出（「令和」「年」「月」「日」を含む）
            if '令和' in token_text or token_text == '令':
                results['date'] = {
                    'marker': '令和',
                    'marker_coords': coords,
                    'fill_positions': {
                        'year': {
                            'x': coords['normalized']['x'] + coords['normalized']['width'],
                            'y': coords['normalized']['y'],
                        },
                        'month': None,  # 後で「月」の位置から計算
                        'day': None,    # 後で「日」の位置から計算
                    }
                }

            # 「年」「月」「日」の位置を記録
            if results['date'] and token_text == '年':
                results['date']['fill_positions']['year']['width'] = coords['normalized']['x'] - results['date']['fill_positions']['year']['x']
                results['date']['fill_positions']['month'] = {
                    'x': coords['normalized']['x'] + coords['normalized']['width'],
                    'y': coords['normalized']['y'],
                }

            if results['date'] and token_text == '月' and results['date']['fill_positions'].get('month'):
                results['date']['fill_positions']['month']['width'] = coords['normalized']['x'] - results['date']['fill_positions']['month']['x']
                results['date']['fill_positions']['day'] = {
                    'x': coords['normalized']['x'] + coords['normalized']['width'],
                    'y': coords['normalized']['y'],
                }

            if results['date'] and token_text == '日' and results['date']['fill_positions'].get('day'):
                results['date']['fill_positions']['day']['width'] = coords['normalized']['x'] - results['date']['fill_positions']['day']['x']

    return results

def check_document(pdf_url, contractor_name, doc_type):
    """ドキュメントをチェックして空欄座標を取得"""
    print(f"\n{'='*60}")
    print(f"業者: {contractor_name}")
    print(f"書類: {doc_type}")
    print(f"{'='*60}")

    # 設定読み込み
    config = load_documentai_config()
    client = get_documentai_client(config)
    drive_service = get_drive_service()

    # ファイル取得
    file_id = extract_file_id(pdf_url)
    if not file_id:
        print("ファイルID抽出失敗")
        return None

    print(f"ファイルID: {file_id}")
    print("PDFダウンロード中...")
    pdf_content = download_pdf(drive_service, file_id)
    print(f"ダウンロード完了: {len(pdf_content)} bytes")

    # Document AI処理
    print("Document AI OCR処理中...")
    document = process_pdf_with_documentai(client, config, pdf_content)
    print(f"OCR完了: {len(document.pages)}ページ")

    # 空欄座標検出
    results = find_blank_field_coordinates(document)

    # 検出されたトークンをファイルに保存（デバッグ用）
    debug_path = Path(r"H:\マイドライブ\〇市道 南千反畑町第１号線舗装補修工事\５施工体制") / f"debug_tokens_{contractor_name}_{doc_type.replace('/', '_')}.json"
    with open(debug_path, 'w', encoding='utf-8') as f:
        json.dump(results['all_tokens'], f, ensure_ascii=False, indent=2)
    print(f"\n検出トークン数: {len(results['all_tokens'])} (保存: {debug_path.name})")

    # 結果表示
    if results['destination']:
        dest = results['destination']
        print(f"\n提出先フィールド検出:")
        print(f"  マーカー: 「{dest['marker']}」")
        print(f"  マーカー位置: x={dest['marker_coords']['normalized']['x']:.4f}, y={dest['marker_coords']['normalized']['y']:.4f}")
        print(f"  記入位置: x={dest['fill_position']['x']:.4f}, y={dest['fill_position']['y']:.4f}")
    else:
        print("\n提出先フィールド: 検出されず")

    if results['date']:
        date = results['date']
        print(f"\n日付フィールド検出:")
        print(f"  マーカー: 「{date['marker']}」")
        print(f"  年の記入位置: x={date['fill_positions']['year']['x']:.4f}")
        if date['fill_positions'].get('month'):
            print(f"  月の記入位置: x={date['fill_positions']['month']['x']:.4f}")
        if date['fill_positions'].get('day'):
            print(f"  日の記入位置: x={date['fill_positions']['day']['x']:.4f}")
    else:
        print("\n日付フィールド: 検出されず")

    return results

def main():
    # sekoutaisei.jsonから対象書類を読み込み
    sekoutaisei_path = Path(r"H:\マイドライブ\〇市道 南千反畑町第１号線舗装補修工事\５施工体制\sekoutaisei.json")

    with open(sekoutaisei_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    all_results = []

    for contractor in data.get('contractors', []):
        contractor_name = contractor.get('name', '')

        for doc_key in ['09_暴対法誓約書', '08_作業員名簿']:
            doc_info = contractor.get('docs', {}).get(doc_key, {})

            if not doc_info.get('status') or not doc_info.get('url'):
                continue

            url = doc_info['url']

            # スプレッドシートはスキップ
            if 'spreadsheets' in url:
                print(f"\n{contractor_name} {doc_key}: スプレッドシート形式のためスキップ")
                continue

            try:
                result = check_document(url, contractor_name, doc_key)
                if result:
                    all_results.append({
                        'contractor': contractor_name,
                        'doc_type': doc_key,
                        'url': url,
                        'coordinates': result
                    })
            except Exception as e:
                print(f"エラー: {e}")

    # 結果をJSONで保存
    output_path = Path(r"H:\マイドライブ\〇市道 南千反畑町第１号線舗装補修工事\５施工体制\blank_field_coordinates.json")
    with open(output_path, 'w', encoding='utf-8') as f:
        # all_tokensは大きいので除外
        save_results = []
        for r in all_results:
            save_r = r.copy()
            if 'coordinates' in save_r and 'all_tokens' in save_r['coordinates']:
                del save_r['coordinates']['all_tokens']
            save_results.append(save_r)
        json.dump(save_results, f, ensure_ascii=False, indent=2)
    print(f"\n結果を保存: {output_path}")

if __name__ == '__main__':
    main()
