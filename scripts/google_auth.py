#!/usr/bin/env python3
"""
Google API認証モジュール
OAuth認証を使用してDrive/Sheets APIにアクセス
"""

import json
from pathlib import Path
from google.oauth2.credentials import Credentials
from google.auth.transport.requests import Request
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build

# デフォルトのトークンパス
DEFAULT_TOKEN_PATH = Path(r'C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\config\gmail_token.json')
DEFAULT_CREDENTIALS_PATH = Path(r'C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\config\credentials.json')

SCOPES = [
    'https://www.googleapis.com/auth/spreadsheets',
    'https://www.googleapis.com/auth/drive',
]


def get_credentials(token_path: Path = None, credentials_path: Path = None) -> Credentials:
    """OAuth認証情報を取得"""
    token_path = token_path or DEFAULT_TOKEN_PATH
    credentials_path = credentials_path or DEFAULT_CREDENTIALS_PATH

    creds = None

    if token_path.exists():
        with open(token_path, 'r') as f:
            token_data = json.load(f)

        creds = Credentials(
            token=token_data.get('token'),
            refresh_token=token_data.get('refresh_token'),
            token_uri=token_data.get('token_uri'),
            client_id=token_data.get('client_id'),
            client_secret=token_data.get('client_secret'),
            scopes=token_data.get('scopes', SCOPES)
        )

    # トークンが無効または期限切れの場合
    if not creds or not creds.valid:
        if creds and creds.expired and creds.refresh_token:
            try:
                creds.refresh(Request())
            except Exception:
                creds = None

        if not creds:
            if not credentials_path.exists():
                raise FileNotFoundError(f"認証ファイルが見つかりません: {credentials_path}")

            flow = InstalledAppFlow.from_client_secrets_file(
                str(credentials_path), SCOPES
            )
            creds = flow.run_local_server(port=0)

        # トークンを保存
        token_data = {
            'token': creds.token,
            'refresh_token': creds.refresh_token,
            'token_uri': creds.token_uri,
            'client_id': creds.client_id,
            'client_secret': creds.client_secret,
            'scopes': list(creds.scopes) if creds.scopes else SCOPES
        }
        with open(token_path, 'w') as f:
            json.dump(token_data, f)

    return creds


def get_sheets_service(creds: Credentials = None):
    """Sheets APIサービスを取得"""
    if creds is None:
        creds = get_credentials()
    return build('sheets', 'v4', credentials=creds)


def get_drive_service(creds: Credentials = None):
    """Drive APIサービスを取得"""
    if creds is None:
        creds = get_credentials()
    return build('drive', 'v3', credentials=creds)


class GoogleAPI:
    """Google API操作クラス"""

    def __init__(self, token_path: Path = None):
        self.creds = get_credentials(token_path)
        self._sheets = None
        self._drive = None

    @property
    def sheets(self):
        if self._sheets is None:
            self._sheets = get_sheets_service(self.creds)
        return self._sheets

    @property
    def drive(self):
        if self._drive is None:
            self._drive = get_drive_service(self.creds)
        return self._drive

    def get_spreadsheet(self, spreadsheet_id: str):
        """スプレッドシートのメタデータを取得"""
        return self.sheets.spreadsheets().get(spreadsheetId=spreadsheet_id).execute()

    def get_values(self, spreadsheet_id: str, range_name: str):
        """セルの値を取得"""
        result = self.sheets.spreadsheets().values().get(
            spreadsheetId=spreadsheet_id,
            range=range_name
        ).execute()
        return result.get('values', [])

    def update_values(self, spreadsheet_id: str, range_name: str, values: list,
                      value_input_option: str = 'USER_ENTERED'):
        """セルの値を更新"""
        return self.sheets.spreadsheets().values().update(
            spreadsheetId=spreadsheet_id,
            range=range_name,
            valueInputOption=value_input_option,
            body={'values': values}
        ).execute()

    def batch_update_values(self, spreadsheet_id: str, data: list,
                           value_input_option: str = 'USER_ENTERED'):
        """複数セルの値を一括更新"""
        return self.sheets.spreadsheets().values().batchUpdate(
            spreadsheetId=spreadsheet_id,
            body={
                'valueInputOption': value_input_option,
                'data': data
            }
        ).execute()

    def batch_update(self, spreadsheet_id: str, requests: list):
        """スプレッドシートのバッチ更新（シート追加・削除など）"""
        return self.sheets.spreadsheets().batchUpdate(
            spreadsheetId=spreadsheet_id,
            body={'requests': requests}
        ).execute()

    def copy_file(self, file_id: str, name: str, parent_folder_id: str = None):
        """ファイルをコピー"""
        body = {'name': name}
        if parent_folder_id:
            body['parents'] = [parent_folder_id]

        return self.drive.files().copy(fileId=file_id, body=body).execute()

    def move_file(self, file_id: str, new_parent_id: str):
        """ファイルを移動"""
        file_info = self.drive.files().get(fileId=file_id, fields='parents').execute()
        old_parents = ','.join(file_info.get('parents', []))

        return self.drive.files().update(
            fileId=file_id,
            addParents=new_parent_id,
            removeParents=old_parents
        ).execute()

    def create_folder(self, name: str, parent_id: str = None):
        """フォルダを作成"""
        body = {
            'name': name,
            'mimeType': 'application/vnd.google-apps.folder'
        }
        if parent_id:
            body['parents'] = [parent_id]

        return self.drive.files().create(body=body).execute()

    def search_files(self, query: str, fields: str = "files(id, name, parents)"):
        """ファイルを検索"""
        results = self.drive.files().list(q=query, fields=fields).execute()
        return results.get('files', [])

    def get_file(self, file_id: str, fields: str = "id, name, parents"):
        """ファイル情報を取得"""
        return self.drive.files().get(fileId=file_id, fields=fields).execute()


if __name__ == '__main__':
    # テスト
    api = GoogleAPI()
    print("認証成功")
