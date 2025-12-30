#!/usr/bin/env python3
"""
契約書スプレッドシートを読み取るスクリプト
Google Sheets API を使用（APIキー方式）

環境変数 GOOGLE_API_KEY にAPIキーを設定して実行
"""

import os
import json
from pathlib import Path
from googleapiclient.discovery import build

# APIキー（環境変数から取得）
API_KEY = os.environ.get('GOOGLE_API_KEY')
if not API_KEY:
    raise ValueError("環境変数 GOOGLE_API_KEY を設定してください")

def read_spreadsheet(spreadsheet_id: str):
    """スプレッドシートを読み取ってJSONに保存"""
    service = build('sheets', 'v4', developerKey=API_KEY)

    # シート一覧を取得
    spreadsheet = service.spreadsheets().get(
        spreadsheetId=spreadsheet_id
    ).execute()

    result = {
        'spreadsheet_name': spreadsheet['properties']['title'],
        'spreadsheet_id': spreadsheet_id,
        'sheets': {}
    }

    sheets = []
    for sheet in spreadsheet['sheets']:
        title = sheet['properties']['title']
        sheets.append(title)

    # 全シートの内容を取得
    for sheet_name in sheets:
        try:
            data = service.spreadsheets().values().get(
                spreadsheetId=spreadsheet_id,
                range=f"'{sheet_name}'!A:Z"
            ).execute()

            values = data.get('values', [])
            result['sheets'][sheet_name] = {
                'rows': len(values),
                'data': values
            }
        except Exception as e:
            result['sheets'][sheet_name] = {'error': str(e)}

    return result

if __name__ == '__main__':
    # 契約書スプレッドシートID
    CONTRACT_SHEET_ID = 'REDACTED_SHEET_ID'

    data = read_spreadsheet(CONTRACT_SHEET_ID)

    # JSONファイルに保存
    output_path = Path(__file__).parent.parent / 'data' / 'contract_sheet.json'
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f"Saved to: {output_path}")
