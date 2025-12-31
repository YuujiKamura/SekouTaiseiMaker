#!/usr/bin/env python3
"""
スプレッドシート操作モジュール
Linkシート作成、IMPORTRANGE設定など
"""

import re
from typing import Optional
from google_auth import GoogleAPI


class SpreadsheetOps:
    """スプレッドシート操作クラス"""

    def __init__(self, api: GoogleAPI = None):
        self.api = api or GoogleAPI()

    def get_sheet_names(self, spreadsheet_id: str) -> list[str]:
        """シート名一覧を取得"""
        spreadsheet = self.api.get_spreadsheet(spreadsheet_id)
        return [sheet['properties']['title'] for sheet in spreadsheet['sheets']]

    def has_sheet(self, spreadsheet_id: str, sheet_name: str) -> bool:
        """指定名のシートが存在するか"""
        return sheet_name in self.get_sheet_names(spreadsheet_id)

    def add_sheet(self, spreadsheet_id: str, sheet_name: str, index: int = 0) -> dict:
        """シートを追加"""
        requests = [{
            'addSheet': {
                'properties': {
                    'title': sheet_name,
                    'index': index
                }
            }
        }]
        return self.api.batch_update(spreadsheet_id, requests)

    def delete_sheet(self, spreadsheet_id: str, sheet_name: str) -> dict:
        """シートを削除"""
        spreadsheet = self.api.get_spreadsheet(spreadsheet_id)
        sheet_id = None
        for sheet in spreadsheet['sheets']:
            if sheet['properties']['title'] == sheet_name:
                sheet_id = sheet['properties']['sheetId']
                break

        if sheet_id is None:
            raise ValueError(f"シートが見つかりません: {sheet_name}")

        requests = [{'deleteSheet': {'sheetId': sheet_id}}]
        return self.api.batch_update(spreadsheet_id, requests)

    def rename_sheet(self, spreadsheet_id: str, old_name: str, new_name: str) -> dict:
        """シート名を変更"""
        spreadsheet = self.api.get_spreadsheet(spreadsheet_id)
        sheet_id = None
        for sheet in spreadsheet['sheets']:
            if sheet['properties']['title'] == old_name:
                sheet_id = sheet['properties']['sheetId']
                break

        if sheet_id is None:
            raise ValueError(f"シートが見つかりません: {old_name}")

        requests = [{
            'updateSheetProperties': {
                'properties': {
                    'sheetId': sheet_id,
                    'title': new_name
                },
                'fields': 'title'
            }
        }]
        return self.api.batch_update(spreadsheet_id, requests)

    def copy_sheet(self, source_spreadsheet_id: str, source_sheet_name: str,
                   dest_spreadsheet_id: str) -> dict:
        """シートを別のスプレッドシートにコピー"""
        spreadsheet = self.api.get_spreadsheet(source_spreadsheet_id)
        sheet_id = None
        for sheet in spreadsheet['sheets']:
            if sheet['properties']['title'] == source_sheet_name:
                sheet_id = sheet['properties']['sheetId']
                break

        if sheet_id is None:
            raise ValueError(f"シートが見つかりません: {source_sheet_name}")

        return self.api.sheets.spreadsheets().sheets().copyTo(
            spreadsheetId=source_spreadsheet_id,
            sheetId=sheet_id,
            body={'destinationSpreadsheetId': dest_spreadsheet_id}
        ).execute()

    def setup_link_sheet(self, spreadsheet_id: str, mokuji_id: str) -> None:
        """Linkシートをセットアップ（なければ作成、A1に目次IDを設定）"""
        if not self.has_sheet(spreadsheet_id, 'Link'):
            self.add_sheet(spreadsheet_id, 'Link', index=0)
            print(f"  Linkシート作成")

        self.api.update_values(
            spreadsheet_id,
            "'Link'!A1",
            [[mokuji_id]],
            'RAW'
        )
        print(f"  Link!A1 = {mokuji_id}")

    def find_and_replace_formulas(self, spreadsheet_id: str,
                                   old_ref: str, new_ref: str,
                                   sheet_name: str = None) -> int:
        """数式内の参照を置換"""
        spreadsheet = self.api.sheets.spreadsheets().get(
            spreadsheetId=spreadsheet_id,
            includeGridData=True
        ).execute()

        updates = []
        for sheet in spreadsheet['sheets']:
            title = sheet['properties']['title']
            if sheet_name and title != sheet_name:
                continue
            if title == 'Link':
                continue

            if 'data' not in sheet:
                continue

            for grid in sheet['data']:
                start_row = grid.get('startRow', 0)
                start_col = grid.get('startColumn', 0)

                for row_idx, row in enumerate(grid.get('rowData', [])):
                    for col_idx, cell in enumerate(row.get('values', [])):
                        formula = cell.get('userEnteredValue', {}).get('formulaValue', '')
                        if old_ref in formula:
                            new_formula = formula.replace(old_ref, new_ref)
                            col_letter = self._col_to_letter(start_col + col_idx)
                            cell_ref = f"'{title}'!{col_letter}{start_row + row_idx + 1}"
                            updates.append({
                                'range': cell_ref,
                                'values': [[new_formula]]
                            })

        if updates:
            self.api.batch_update_values(spreadsheet_id, updates)

        return len(updates)

    def update_importrange_refs(self, spreadsheet_id: str) -> int:
        """$B$1参照をLink!$A$1に置換"""
        count = 0
        for pattern in ['$B$1', '$B1', 'B$1', 'B1']:
            count += self.find_and_replace_formulas(
                spreadsheet_id, pattern, 'Link!$A$1'
            )
        return count

    def set_cell_formula(self, spreadsheet_id: str, cell_ref: str, formula: str) -> None:
        """セルに数式を設定"""
        self.api.update_values(
            spreadsheet_id,
            cell_ref,
            [[formula]],
            'USER_ENTERED'
        )

    def set_cell_value(self, spreadsheet_id: str, cell_ref: str, value) -> None:
        """セルに値を設定"""
        self.api.update_values(
            spreadsheet_id,
            cell_ref,
            [[value]],
            'RAW'
        )

    def clear_cells(self, spreadsheet_id: str, cell_refs: list[str]) -> None:
        """複数セルをクリア"""
        updates = [{'range': ref, 'values': [['']] } for ref in cell_refs]
        self.api.batch_update_values(spreadsheet_id, updates, 'RAW')

    def get_formulas(self, spreadsheet_id: str, range_name: str) -> list:
        """数式を取得"""
        result = self.api.sheets.spreadsheets().values().get(
            spreadsheetId=spreadsheet_id,
            range=range_name,
            valueRenderOption='FORMULA'
        ).execute()
        return result.get('values', [])

    @staticmethod
    def _col_to_letter(col: int) -> str:
        """列番号をアルファベットに変換（0始まり）"""
        result = ''
        while col >= 0:
            result = chr(65 + col % 26) + result
            col = col // 26 - 1
        return result

    @staticmethod
    def _letter_to_col(letter: str) -> int:
        """アルファベットを列番号に変換（0始まり）"""
        result = 0
        for char in letter.upper():
            result = result * 26 + (ord(char) - ord('A') + 1)
        return result - 1


class TemplateManager:
    """テンプレート管理クラス"""

    # 既知のテンプレート
    TEMPLATES = {
        '下請契約書': {
            'source_project': '沈目舞原',
            'search_name': '建設工事下請契約書',
        },
        '産廃契約書': {
            'source_project': '沈目舞原',
            'search_name': '建設廃棄物処理実施計画書と契約書',
        },
        '施工体制台帳': {
            'source_project': '上古閑',
            'search_name': '施工体制台帳',
        },
    }

    # 過去現場フォルダ
    PAST_PROJECTS_FOLDER = 'H:/マイドライブ/過去の現場_元請'

    def __init__(self, api: GoogleAPI = None):
        self.api = api or GoogleAPI()
        self.ops = SpreadsheetOps(self.api)

    def find_template(self, template_name: str) -> Optional[str]:
        """テンプレートのスプレッドシートIDを検索"""
        if template_name not in self.TEMPLATES:
            raise ValueError(f"未知のテンプレート: {template_name}")

        config = self.TEMPLATES[template_name]
        search_name = config['search_name']

        # Drive APIで検索
        query = f"name contains '{search_name}' and mimeType='application/vnd.google-apps.spreadsheet'"
        files = self.api.search_files(query)

        if files:
            return files[0]['id']
        return None

    def copy_template(self, template_id: str, new_name: str,
                      parent_folder_id: str = None) -> str:
        """テンプレートをコピー"""
        result = self.api.copy_file(template_id, new_name, parent_folder_id)
        return result['id']

    def setup_from_template(self, template_id: str, new_name: str,
                            mokuji_id: str, parent_folder_id: str = None) -> str:
        """テンプレートから新規作成してセットアップ"""
        # コピー
        new_id = self.copy_template(template_id, new_name, parent_folder_id)
        print(f"コピー完了: {new_name}")
        print(f"  新ID: {new_id}")

        # Linkシートセットアップ
        self.ops.setup_link_sheet(new_id, mokuji_id)

        # IMPORTRANGE参照を更新
        count = self.ops.update_importrange_refs(new_id)
        print(f"  IMPORTRANGE更新: {count}箇所")

        return new_id


if __name__ == '__main__':
    # テスト
    api = GoogleAPI()
    ops = SpreadsheetOps(api)
    mgr = TemplateManager(api)

    print("スプレッドシート操作モジュール準備完了")
