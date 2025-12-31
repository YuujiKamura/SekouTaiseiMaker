#!/usr/bin/env python3
"""
書類生成CLI
目次シートIDを指定して各種書類を一発生成
"""

import argparse
import sys
from pathlib import Path

# 同じディレクトリのモジュールをインポート
sys.path.insert(0, str(Path(__file__).parent))

from google_auth import GoogleAPI
from spreadsheet_ops import SpreadsheetOps
from generators import ProjectInfo, SubcontractGenerator, WasteContractGenerator, SekouTaiseiGenerator


def get_project_info(api: GoogleAPI, mokuji_id: str) -> ProjectInfo:
    """目次シートから工事情報を取得"""
    ops = SpreadsheetOps(api)

    # 工事内容シートから情報取得
    values = api.get_values(mokuji_id, "'工事内容'!A1:D20")

    project_name = ""
    for row in values:
        if row and row[0] == '工事名称':
            project_name = row[1] if len(row) > 1 else ""
            break

    return ProjectInfo(
        mokuji_id=mokuji_id,
        project_name=project_name
    )


def cmd_info(args):
    """目次シート情報を表示"""
    api = GoogleAPI()
    project = get_project_info(api, args.mokuji_id)

    print(f"\n=== 工事情報 ===")
    print(f"工事名: {project.project_name}")
    print(f"目次ID: {project.mokuji_id}")

    # シート一覧
    ops = SpreadsheetOps(api)
    sheets = ops.get_sheet_names(args.mokuji_id)
    print(f"\nシート一覧:")
    for s in sheets:
        print(f"  - {s}")


def cmd_waste(args):
    """産廃契約書を生成"""
    api = GoogleAPI()
    project = get_project_info(api, args.mokuji_id)

    print(f"\n=== 産廃契約書生成 ===")
    print(f"工事名: {project.project_name}")

    generator = WasteContractGenerator(api)
    result = generator.generate(
        project,
        contract_date=args.contract_date,
        emission_tonnage=args.tonnage
    )

    # AS塊リンク設定
    if not args.no_as_link:
        generator.setup_as_tonnage_link(result['spreadsheet_id'])

    # カッターなしの場合、汚泥行をクリア
    if args.no_cutter:
        generator.clear_sludge_entries(result['spreadsheet_id'])

    print(f"\n完了!")
    print(f"URL: {result['url']}")


def cmd_subcontract(args):
    """下請契約書を生成"""
    api = GoogleAPI()
    project = get_project_info(api, args.mokuji_id)

    print(f"\n=== 下請契約書生成 ===")
    print(f"工事名: {project.project_name}")

    # 下請業者情報（簡易版）
    subcontractors = []
    if args.subcontractors:
        for s in args.subcontractors:
            parts = s.split(':')
            sub = {'name': parts[0]}
            if len(parts) > 1:
                sub['sekoutaisei_id'] = parts[1]
            subcontractors.append(sub)

    generator = SubcontractGenerator(api)
    result = generator.generate(project, subcontractors)

    print(f"\n完了!")
    print(f"URL: {result['url']}")


def cmd_taicho(args):
    """施工体制台帳を生成"""
    api = GoogleAPI()
    project = get_project_info(api, args.mokuji_id)

    print(f"\n=== 施工体制台帳生成 ===")
    print(f"工事名: {project.project_name}")

    generator = SekouTaiseiGenerator(api)
    result = generator.generate(project)

    print(f"\n完了!")
    print(f"URL: {result['url']}")


def cmd_copy(args):
    """スプレッドシートをコピーしてLink設定"""
    api = GoogleAPI()
    ops = SpreadsheetOps(api)

    print(f"\n=== スプレッドシートコピー ===")
    print(f"コピー元: {args.source_id}")
    print(f"目次ID: {args.mokuji_id}")

    # コピー
    result = api.copy_file(args.source_id, args.name)
    new_id = result['id']
    print(f"コピー完了: {args.name}")
    print(f"  新ID: {new_id}")

    # Linkシート設定
    ops.setup_link_sheet(new_id, args.mokuji_id)

    # IMPORTRANGE更新
    if not args.no_importrange:
        count = ops.update_importrange_refs(new_id)
        print(f"  IMPORTRANGE更新: {count}箇所")

    print(f"\n完了!")
    print(f"URL: https://docs.google.com/spreadsheets/d/{new_id}/edit")


def cmd_link(args):
    """既存スプレッドシートにLink設定"""
    api = GoogleAPI()
    ops = SpreadsheetOps(api)

    print(f"\n=== Linkシート設定 ===")
    print(f"対象: {args.spreadsheet_id}")
    print(f"目次ID: {args.mokuji_id}")

    ops.setup_link_sheet(args.spreadsheet_id, args.mokuji_id)

    if not args.no_importrange:
        count = ops.update_importrange_refs(args.spreadsheet_id)
        print(f"  IMPORTRANGE更新: {count}箇所")

    print(f"\n完了!")


def cmd_search(args):
    """スプレッドシートを検索"""
    api = GoogleAPI()

    query = f"name contains '{args.query}' and mimeType='application/vnd.google-apps.spreadsheet'"
    files = api.search_files(query)

    print(f"\n=== 検索結果: {args.query} ===")
    for f in files[:20]:
        print(f"  {f['name']}")
        print(f"    ID: {f['id']}")


def main():
    parser = argparse.ArgumentParser(
        description='書類生成ツール',
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    subparsers = parser.add_subparsers(dest='command', help='コマンド')

    # info
    p_info = subparsers.add_parser('info', help='目次シート情報を表示')
    p_info.add_argument('mokuji_id', help='目次スプレッドシートID')
    p_info.set_defaults(func=cmd_info)

    # waste - 産廃契約書
    p_waste = subparsers.add_parser('waste', help='産廃契約書を生成')
    p_waste.add_argument('mokuji_id', help='目次スプレッドシートID')
    p_waste.add_argument('--contract-date', '-d', help='契約日（例: 令和８年１月７日）')
    p_waste.add_argument('--tonnage', '-t', type=float, help='搬出量（トン）')
    p_waste.add_argument('--no-cutter', action='store_true', help='カッター工事なし（汚泥行をクリア）')
    p_waste.add_argument('--no-as-link', action='store_true', help='AS塊リンクを設定しない')
    p_waste.set_defaults(func=cmd_waste)

    # subcontract - 下請契約書
    p_sub = subparsers.add_parser('subcontract', help='下請契約書を生成')
    p_sub.add_argument('mokuji_id', help='目次スプレッドシートID')
    p_sub.add_argument('--subcontractors', '-s', nargs='+', help='下請業者（名前:施工体制ID）')
    p_sub.set_defaults(func=cmd_subcontract)

    # taicho - 施工体制台帳
    p_taicho = subparsers.add_parser('taicho', help='施工体制台帳を生成')
    p_taicho.add_argument('mokuji_id', help='目次スプレッドシートID')
    p_taicho.set_defaults(func=cmd_taicho)

    # copy - 汎用コピー
    p_copy = subparsers.add_parser('copy', help='スプレッドシートをコピー')
    p_copy.add_argument('source_id', help='コピー元スプレッドシートID')
    p_copy.add_argument('mokuji_id', help='目次スプレッドシートID')
    p_copy.add_argument('name', help='新しいファイル名')
    p_copy.add_argument('--no-importrange', action='store_true', help='IMPORTRANGE更新をスキップ')
    p_copy.set_defaults(func=cmd_copy)

    # link - Link設定のみ
    p_link = subparsers.add_parser('link', help='既存シートにLink設定')
    p_link.add_argument('spreadsheet_id', help='対象スプレッドシートID')
    p_link.add_argument('mokuji_id', help='目次スプレッドシートID')
    p_link.add_argument('--no-importrange', action='store_true', help='IMPORTRANGE更新をスキップ')
    p_link.set_defaults(func=cmd_link)

    # search - 検索
    p_search = subparsers.add_parser('search', help='スプレッドシートを検索')
    p_search.add_argument('query', help='検索クエリ')
    p_search.set_defaults(func=cmd_search)

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return

    args.func(args)


if __name__ == '__main__':
    main()
