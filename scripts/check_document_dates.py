"""
施工体制書類の提出先・提出日チェックスクリプト
- 提出先名称の空欄チェック
- 提出日/作成日の空欄チェック
- 工期との整合性チェック
"""

import json
import re
import base64
import requests
from pathlib import Path
from datetime import datetime
from google.oauth2.credentials import Credentials
import googleapiclient.discovery

from gemini_checker import GEMINI_MODEL_NAME, get_api_key

# 設定
PROJECT_ROOT = Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator")
TOKEN_PATH = PROJECT_ROOT / "gmail_token.json"

SCOPES = [
    'https://www.googleapis.com/auth/drive',
    'https://www.googleapis.com/auth/spreadsheets',
    'https://mail.google.com/',
]

# チェック対象の書類タイプ
CHECK_TARGET_DOCS = [
    "08_作業員名簿",
    "09_暴対法誓約書",
]

def get_drive_service():
    """Drive APIサービスを取得"""
    from google.auth.transport.requests import Request

    creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)

    # トークンをリフレッシュ（常に最新に）
    if creds and creds.refresh_token:
        try:
            creds.refresh(Request())
            # 更新されたトークンを保存
            with open(TOKEN_PATH, 'w') as token:
                token.write(creds.to_json())
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

def download_file_as_base64(drive_service, file_id):
    """ファイルをダウンロードしてBase64エンコード"""
    request = drive_service.files().get_media(fileId=file_id)
    content = request.execute()
    return base64.standard_b64encode(content).decode('utf-8')

def get_file_mime_type(drive_service, file_id):
    """ファイルのMIMEタイプを取得"""
    file_info = drive_service.files().get(fileId=file_id, fields='mimeType,name').execute()
    return file_info.get('mimeType'), file_info.get('name')

def read_document_with_gemini(file_base64, mime_type, prompt):
    """Gemini APIでドキュメントを読み取る"""
    api_key = get_api_key()
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{GEMINI_MODEL_NAME}:generateContent?key={api_key}"

    payload = {
        "contents": [{
            "parts": [
                {
                    "inline_data": {
                        "mime_type": mime_type,
                        "data": file_base64
                    }
                },
                {"text": prompt}
            ]
        }]
    }

    response = requests.post(url, json=payload)
    result = response.json()

    if 'candidates' in result and result['candidates']:
        return result['candidates'][0]['content']['parts'][0]['text']
    return None

def parse_period(period_str):
    """工期文字列をパース (例: 令和7年1月〜令和7年3月)"""
    # 令和年を西暦に変換
    def reiwa_to_year(reiwa_year):
        return 2018 + int(reiwa_year)

    pattern = r'令和(\d+)年(\d+)月[〜～\-]令和(\d+)年(\d+)月'
    match = re.search(pattern, period_str)
    if match:
        start_year = reiwa_to_year(match.group(1))
        start_month = int(match.group(2))
        end_year = reiwa_to_year(match.group(3))
        end_month = int(match.group(4))
        return (start_year, start_month), (end_year, end_month)
    return None, None

def parse_date(date_str):
    """日付文字列をパース"""
    if not date_str or date_str.strip() in ['', '空欄', '記載なし', 'なし', 'N/A', 'null', 'None']:
        return None

    # 令和形式
    match = re.search(r'令和(\d+)年(\d+)月(\d+)日', date_str)
    if match:
        year = 2018 + int(match.group(1))
        month = int(match.group(2))
        day = int(match.group(3))
        return (year, month, day)

    # 西暦形式
    match = re.search(r'(\d{4})年(\d+)月(\d+)日', date_str)
    if match:
        return (int(match.group(1)), int(match.group(2)), int(match.group(3)))

    return None

def check_date_in_period(date_tuple, period_start, period_end, margin_months=3):
    """日付が工期内（±マージン）かチェック"""
    if not date_tuple or not period_start or not period_end:
        return None

    year, month, _ = date_tuple
    start_year, start_month = period_start
    end_year, end_month = period_end

    # 工期開始の3ヶ月前から工期終了の3ヶ月後まで許容
    earliest_month = (start_year * 12 + start_month) - margin_months
    latest_month = (end_year * 12 + end_month) + margin_months
    doc_month = year * 12 + month

    if doc_month < earliest_month:
        return "too_early"
    elif doc_month > latest_month:
        return "too_late"
    return "ok"

def check_documents(sekoutaisei_path):
    """書類をチェック"""
    with open(sekoutaisei_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    project_name = data.get('project_name', '')
    period = data.get('period', '')
    period_start, period_end = parse_period(period)

    print(f"工事名: {project_name}")
    print(f"工期: {period}")
    if period_start and period_end:
        print(f"  → {period_start[0]}年{period_start[1]}月 〜 {period_end[0]}年{period_end[1]}月")
    print()

    drive_service = get_drive_service()
    results = []

    for contractor in data.get('contractors', []):
        contractor_name = contractor.get('name', '')
        print(f"=== {contractor_name} ===")

        for doc_key, doc_info in contractor.get('docs', {}).items():
            if doc_key not in CHECK_TARGET_DOCS:
                continue

            if not doc_info.get('status') or not doc_info.get('url'):
                print(f"  {doc_key}: スキップ（未提出またはURL無し）")
                continue

            url = doc_info['url']
            file_id = extract_file_id(url)
            if not file_id:
                print(f"  {doc_key}: ファイルID抽出失敗")
                continue

            print(f"  {doc_key}: チェック中...")

            try:
                mime_type, file_name = get_file_mime_type(drive_service, file_id)
                print(f"    ファイル: {file_name} ({mime_type})")

                # スプレッドシートはスキップ（PDFのみ対象）
                if 'spreadsheet' in mime_type or '.xlsx' in (file_name or ''):
                    print(f"    → スプレッドシート形式のためスキップ")
                    continue

                file_base64 = download_file_as_base64(drive_service, file_id)

                prompt = """この書類から以下の情報を抽出してください:

1. 提出先（宛先）: 書類の冒頭にある「○○殿」や「○○様」の部分。空欄の場合は「空欄」と回答。
2. 提出日/作成日: 書類の右上や署名欄付近にある日付。空欄の場合は「空欄」と回答。

JSON形式で回答:
{
  "destination": "提出先の名称（空欄の場合は「空欄」）",
  "date": "日付（令和X年X月X日形式、空欄の場合は「空欄」）"
}"""

                response = read_document_with_gemini(file_base64, mime_type, prompt)
                print(f"    Gemini応答: {response}")

                # JSON抽出
                json_match = re.search(r'\{[^}]+\}', response, re.DOTALL)
                if json_match:
                    parsed = json.loads(json_match.group())
                    destination = parsed.get('destination', '')
                    date_str = parsed.get('date', '')

                    result = {
                        'contractor': contractor_name,
                        'doc_type': doc_key,
                        'file': file_name,
                        'destination': destination,
                        'date': date_str,
                        'warnings': []
                    }

                    # 提出先チェック（空欄・殿のみ・様のみなどを検出）
                    dest_is_empty = (
                        not destination or
                        destination in ['空欄', '記載なし', 'なし', '空', '殿', '様', '御中'] or
                        destination.strip() in ['殿', '様', '御中'] or
                        len(destination.replace('殿', '').replace('様', '').replace('御中', '').strip()) == 0
                    )
                    if dest_is_empty:
                        result['warnings'].append('提出先が空欄または未記入')
                        print(f"    [!] 警告: 提出先が空欄または未記入 (値: {destination})")

                    # 日付チェック（空欄・年月日のみなどを検出）
                    date_is_empty = (
                        not date_str or
                        date_str in ['空欄', '記載なし', 'なし', '空'] or
                        # 「令和　年　月　日」のような空欄パターン
                        re.match(r'^令和\s*年\s*月\s*日$', date_str.replace('\t', ' ')) or
                        # 数字がないパターン
                        not re.search(r'\d', date_str)
                    )
                    if date_is_empty:
                        result['warnings'].append(f'提出日が空欄または未記入 ({date_str})')
                        print(f"    [!] 警告: 提出日が空欄または未記入 (値: {date_str})")
                    else:
                        date_tuple = parse_date(date_str)
                        if date_tuple:
                            period_check = check_date_in_period(date_tuple, period_start, period_end)
                            if period_check == 'too_early':
                                result['warnings'].append(f'日付が工期より著しく早い ({date_str})')
                                print(f"    [!] 警告: 日付が工期より著しく早い ({date_str})")
                            elif period_check == 'too_late':
                                result['warnings'].append(f'日付が工期より著しく遅い ({date_str})')
                                print(f"    [!] 警告: 日付が工期より著しく遅い ({date_str})")
                            else:
                                print(f"    [OK] 日付: {date_str}")

                    results.append(result)

            except Exception as e:
                print(f"    エラー: {e}")

        print()

    return results

def main():
    sekoutaisei_path = Path(r"H:\マイドライブ\〇市道 南千反畑町第１号線舗装補修工事\５施工体制\sekoutaisei.json")

    print("=" * 60)
    print("施工体制書類 提出先・日付チェック")
    print("=" * 60)
    print()

    results = check_documents(sekoutaisei_path)

    print("=" * 60)
    print("チェック結果サマリー")
    print("=" * 60)

    warnings_found = False
    for r in results:
        if r['warnings']:
            warnings_found = True
            print(f"\n【{r['contractor']}】{r['doc_type']}")
            print(f"  ファイル: {r['file']}")
            print(f"  提出先: {r['destination']}")
            print(f"  日付: {r['date']}")
            for w in r['warnings']:
                print(f"  [!] {w}")

    if not warnings_found:
        print("\n警告なし: すべての書類が正常です")

    # JSONファイルに結果を保存
    output_path = Path(r"H:\マイドライブ\〇市道 南千反畑町第１号線舗装補修工事\５施工体制\document_check_results.json")
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump({
            'check_date': datetime.now().isoformat(),
            'project_name': '市道 南千反畑町第１号線舗装補修工事',
            'results': results
        }, f, ensure_ascii=False, indent=2)
    print(f"\n結果を保存: {output_path}")

    return results

if __name__ == '__main__':
    main()
