#!/usr/bin/env python3
"""
Google DriveフォルダからスプレッドシートIDを取得
"""

import json
from pathlib import Path
from google.oauth2.credentials import Credentials
from googleapiclient.discovery import build

def get_credentials():
    """clasp認証情報を取得"""
    clasprc_path = Path.home() / '.clasprc.json'
    with open(clasprc_path, 'r') as f:
        clasp_data = json.load(f)

    tokens = clasp_data.get('tokens', {})
    oauth2_data = tokens.get('default', tokens)

    return Credentials(
        token=oauth2_data.get('access_token'),
        refresh_token=oauth2_data.get('refresh_token'),
        token_uri='https://oauth2.googleapis.com/token',
        client_id=oauth2_data.get('client_id'),
        client_secret=oauth2_data.get('client_secret')
    )

def list_spreadsheets_in_folder(folder_name: str):
    """フォルダ内のスプレッドシートを一覧"""
    creds = get_credentials()
    service = build('drive', 'v3', credentials=creds)

    # フォルダを検索
    query = f"name = '{folder_name}' and mimeType = 'application/vnd.google-apps.folder'"
    results = service.files().list(q=query, fields="files(id, name)").execute()
    folders = results.get('files', [])

    if not folders:
        print(f"フォルダが見つかりません: {folder_name}")
        return []

    folder_id = folders[0]['id']
    print(f"フォルダ: {folders[0]['name']} (ID: {folder_id})")

    # フォルダ内のスプレッドシートを検索
    query = f"'{folder_id}' in parents and mimeType = 'application/vnd.google-apps.spreadsheet'"
    results = service.files().list(q=query, fields="files(id, name, webViewLink)").execute()
    sheets = results.get('files', [])

    print(f"\nスプレッドシート一覧 ({len(sheets)}件):")
    for sheet in sheets:
        print(f"  - {sheet['name']}")
        print(f"    ID: {sheet['id']}")
        print(f"    URL: {sheet['webViewLink']}")

    return sheets

if __name__ == '__main__':
    list_spreadsheets_in_folder('５施工体制')
