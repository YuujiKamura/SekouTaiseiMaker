#!/usr/bin/env python3
"""
書類生成モジュール
下請契約書、産廃契約書などの生成
"""

from dataclasses import dataclass
from typing import Optional
from google_auth import GoogleAPI
from spreadsheet_ops import SpreadsheetOps, TemplateManager


@dataclass
class ProjectInfo:
    """工事情報"""
    mokuji_id: str  # 目次スプレッドシートID
    project_name: str  # 工事名
    parent_folder_id: str = None  # 配置先フォルダID


class SubcontractGenerator:
    """下請契約書生成"""

    # 下請契約書テンプレートID
    TEMPLATE_ID = '13DpLrdgNMtfZhqkX69YiKCV0LHvOp4UpI434piaplk8'

    def __init__(self, api: GoogleAPI = None):
        self.api = api or GoogleAPI()
        self.ops = SpreadsheetOps(self.api)

    def find_template(self) -> str:
        """下請契約書テンプレートを検索"""
        # 固定テンプレートを使用
        return self.TEMPLATE_ID

    def generate(self, project: ProjectInfo, subcontractors: list[dict]) -> dict:
        """
        下請契約書を生成

        Args:
            project: 工事情報
            subcontractors: 下請業者リスト [{'name': '業者名', 'sekoutaisei_id': 'ID'}, ...]

        Returns:
            {'spreadsheet_id': str, 'url': str}
        """
        template_id = self.find_template()

        # コピー
        new_name = f"建設工事下請契約書、{project.project_name.replace('市道 ', '')}"
        result = self.api.copy_file(template_id, new_name, project.parent_folder_id)
        new_id = result['id']
        print(f"下請契約書作成: {new_name}")

        # Linkシートセットアップ
        self.ops.setup_link_sheet(new_id, project.mokuji_id)

        # IMPORTRANGE参照を更新
        self.ops.update_importrange_refs(new_id)

        # 不要なシートを削除（既存のサンプル業者）
        existing_sheets = self.ops.get_sheet_names(new_id)
        keep_sheets = ['Link', '様式']  # 保持するシート

        for sheet in existing_sheets:
            if sheet not in keep_sheets and not any(s['name'] in sheet for s in subcontractors):
                try:
                    self.ops.delete_sheet(new_id, sheet)
                    print(f"  シート削除: {sheet}")
                except:
                    pass

        # 目次シートに業者IDを登録
        self._register_subcontractors(project.mokuji_id, subcontractors)

        return {
            'spreadsheet_id': new_id,
            'url': f"https://docs.google.com/spreadsheets/d/{new_id}/edit"
        }

    def _register_subcontractors(self, mokuji_id: str, subcontractors: list[dict]):
        """目次シートに下請業者IDを登録"""
        # 施工体制シートのB35以降に登録
        start_row = 35
        updates = []
        for i, sub in enumerate(subcontractors):
            if 'sekoutaisei_id' in sub:
                updates.append({
                    'range': f"'施工体制'!B{start_row + i * 2}",
                    'values': [[sub['sekoutaisei_id']]]
                })
        if updates:
            self.api.batch_update_values(mokuji_id, updates, 'RAW')


class WasteContractGenerator:
    """建設廃棄物処理委託契約書生成"""

    # 産廃契約書テンプレートID
    TEMPLATE_ID = '1M8rZLpMZzpi_4PfZi3rBK-Uj5IQIP7RJfIRF8oxD0sg'

    def __init__(self, api: GoogleAPI = None):
        self.api = api or GoogleAPI()
        self.ops = SpreadsheetOps(self.api)

    def find_template(self) -> str:
        """産廃契約書テンプレートを取得"""
        return self.TEMPLATE_ID

    def generate(self, project: ProjectInfo, contract_date: str = None,
                 emission_tonnage: float = None) -> dict:
        """
        建設廃棄物処理委託契約書を生成

        Args:
            project: 工事情報
            contract_date: 契約日（例: "令和　８年　１月　７日"）
            emission_tonnage: 搬出量（トン）

        Returns:
            {'spreadsheet_id': str, 'url': str}
        """
        template_id = self.find_template()

        # コピー
        short_name = project.project_name.replace('市道 ', '').replace('舗装補修工事', '')
        new_name = f"建設廃棄物処理委託契約書、{short_name}"
        result = self.api.copy_file(template_id, new_name, project.parent_folder_id)
        new_id = result['id']
        print(f"産廃契約書作成: {new_name}")

        # Linkシートセットアップ（なければ作成）
        if not self.ops.has_sheet(new_id, 'Link'):
            self.ops.add_sheet(new_id, 'Link', index=0)

        self.api.update_values(new_id, "'Link'!A1", [[project.mokuji_id]], 'RAW')
        print(f"  Link!A1 = {project.mokuji_id}")

        # IMPORTRANGE参照を更新（$B$1 → Link!$A$1）
        count = self.ops.update_importrange_refs(new_id)
        print(f"  IMPORTRANGE更新: {count}箇所")

        # 建設廃棄物処理実施計画書のIMPORTRANGE設定
        self._setup_plan_sheet_formulas(new_id)

        # 入力シートの更新
        if contract_date or emission_tonnage:
            self._update_input_sheet(new_id, project.mokuji_id, contract_date, emission_tonnage)

        return {
            'spreadsheet_id': new_id,
            'url': f"https://docs.google.com/spreadsheets/d/{new_id}/edit"
        }

    def _setup_plan_sheet_formulas(self, spreadsheet_id: str):
        """建設廃棄物処理実施計画書のIMPORTRANGE数式を設定"""
        formulas = [
            # 担当課、監督員、現場代理人
            ("'建設廃棄物処理実施計画書'!L5", '=IMPORTRANGE(Link!$A$1,"工事内容!B11")'),
            ("'建設廃棄物処理実施計画書'!V5", '=IMPORTRANGE(Link!$A$1,"工事内容!B14")'),
            ("'建設廃棄物処理実施計画書'!I10", '=IMPORTRANGE(Link!$A$1,"施工体制!B19")'),
            # 工期
            ("'建設廃棄物処理実施計画書'!Q17", '=IMPORTRANGE(Link!$A$1,"工事内容!B6")'),
            ("'建設廃棄物処理実施計画書'!Q18", '=IMPORTRANGE(Link!$A$1,"工事内容!B7")'),
        ]

        updates = [{'range': cell, 'values': [[formula]]} for cell, formula in formulas]
        self.api.batch_update_values(spreadsheet_id, updates, 'USER_ENTERED')
        print(f"  計画書IMPORTRANGE設定: {len(formulas)}箇所")

    def _update_input_sheet(self, spreadsheet_id: str, mokuji_id: str,
                            contract_date: str = None, tonnage: float = None):
        """入力シートを更新"""
        updates = []

        # 目次ID参照
        updates.extend([
            {'range': "'入力シート (2)'!M3", 'values': [["=Link!$A$1"]]},
            {'range': "'入力シート (2)'!M7", 'values': [["=Link!$A$1"]]},
        ])

        # 契約日
        if contract_date:
            updates.extend([
                {'range': "'入力シート (2)'!B3", 'values': [[contract_date]]},
                {'range': "'入力シート (2)'!B7", 'values': [[contract_date]]},
            ])

        # 搬出量
        if tonnage:
            updates.extend([
                {'range': "'入力シート (2)'!W3", 'values': [[tonnage]]},
                {'range': "'入力シート (2)'!W7", 'values': [[tonnage]]},
            ])

        if updates:
            self.api.batch_update_values(spreadsheet_id, updates, 'USER_ENTERED')
            print(f"  入力シート更新: {len(updates)}箇所")

    def setup_as_tonnage_link(self, spreadsheet_id: str):
        """AS塊の数量を入力シートにリンク"""
        formulas = [
            ("'建設廃棄物処理実施計画書'!C17", "='入力シート (2)'!W7&\"　TON\""),
            ("'建設廃棄物処理実施計画書'!I17", "='入力シート (2)'!W7&\"　TON\""),
            ("'建設廃棄物処理実施計画書'!M17", "='入力シート (2)'!W7&\"　TON\""),
        ]
        updates = [{'range': cell, 'values': [[formula]]} for cell, formula in formulas]
        self.api.batch_update_values(spreadsheet_id, updates, 'USER_ENTERED')
        print(f"  AS塊数量リンク設定")

    def clear_sludge_entries(self, spreadsheet_id: str):
        """建設汚泥（カッター工事なし）の行をクリア"""
        cells_to_clear = [
            "'建設廃棄物処理実施計画書'!C15",
            "'建設廃棄物処理実施計画書'!I15",
            "'建設廃棄物処理実施計画書'!M15",
            "'建設廃棄物処理実施計画書'!Q15",
            "'建設廃棄物処理実施計画書'!T15",
            # 前田カッター関連
            "'建設廃棄物処理実施計画書'!B25",
            "'建設廃棄物処理実施計画書'!D25",
            "'建設廃棄物処理実施計画書'!G25",
            "'建設廃棄物処理実施計画書'!J25",
            "'建設廃棄物処理実施計画書'!M25",
            "'建設廃棄物処理実施計画書'!P25",
            "'建設廃棄物処理実施計画書'!S25",
            "'建設廃棄物処理実施計画書'!V25",
        ]
        self.ops.clear_cells(spreadsheet_id, cells_to_clear)
        print(f"  建設汚泥・前田カッター行クリア")


class SekouTaiseiGenerator:
    """施工体制台帳生成"""

    # 施工体制台帳テンプレートID
    TEMPLATE_ID = '1YyF-nWjIS8qRfnzDNktFiKbLpotsQQewLVGGDfZMyF0'

    def __init__(self, api: GoogleAPI = None):
        self.api = api or GoogleAPI()
        self.ops = SpreadsheetOps(self.api)

    def find_template(self) -> str:
        """施工体制台帳テンプレートを取得"""
        return self.TEMPLATE_ID

    def generate(self, project: ProjectInfo) -> dict:
        """施工体制台帳を生成"""
        template_id = self.find_template()

        short_name = project.project_name.replace('市道 ', '').replace('舗装補修工事', '')
        new_name = f"施工体制台帳、{short_name}"
        result = self.api.copy_file(template_id, new_name, project.parent_folder_id)
        new_id = result['id']
        print(f"施工体制台帳作成: {new_name}")

        # Linkシートセットアップ
        self.ops.setup_link_sheet(new_id, project.mokuji_id)

        return {
            'spreadsheet_id': new_id,
            'url': f"https://docs.google.com/spreadsheets/d/{new_id}/edit"
        }


if __name__ == '__main__':
    # テスト
    api = GoogleAPI()

    # プロジェクト情報の例
    project = ProjectInfo(
        mokuji_id='1uy4XgalfnwiQjHwckULAQM25JIW0AMMmyo0_r8YN5eI',
        project_name='市道 南千反畑町第１号線舗装補修工事'
    )

    print("ジェネレーター準備完了")
    print(f"プロジェクト: {project.project_name}")
    print(f"目次ID: {project.mokuji_id}")
