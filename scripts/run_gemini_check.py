#!/usr/bin/env python3
"""
CLIからGEMINIチェックを実行

使用例:
    # PDF画像をチェック
    python run_gemini_check.py \\
        --type pdf \\
        --path "data/pdf_images/ユナイト_09_暴対法誓約書.png" \\
        --doc-type "暴対法誓約書" \\
        --contractor "ユナイト"

    # 複数ページのPDF画像をチェック
    python run_gemini_check.py \\
        --type pdf \\
        --path "data/pdf_images/page1.png" "data/pdf_images/page2.png" \\
        --doc-type "作業員名簿" \\
        --contractor "アイエスティー"

    # スプレッドシートをチェック
    python run_gemini_check.py \\
        --type spreadsheet \\
        --spreadsheet-id "1Tm6alT13Jno_Fcq0Ml5OvPh9RqKNkIdXxhoVRqK--a8" \\
        --doc-type "作業員名簿" \\
        --contractor "アイエスティー"
"""
import argparse
import json
import sys
from pathlib import Path

# スクリプトディレクトリをパスに追加
sys.path.insert(0, str(Path(__file__).parent))

from gemini_checker import check_pdf_image, check_spreadsheet, check_multiple_pages
from document_prompts import DOC_TYPES


def print_result(result: dict, color: bool = True) -> None:
    """結果を見やすく表示"""
    status = result.get("status", "unknown")
    summary = result.get("summary", "")
    items = result.get("items", [])
    missing_fields = result.get("missing_fields", [])

    # ステータスに応じた色
    if color:
        colors = {
            "ok": "\033[92m",      # 緑
            "warning": "\033[93m", # 黄
            "error": "\033[91m",   # 赤
            "reset": "\033[0m"
        }
    else:
        colors = {"ok": "", "warning": "", "error": "", "reset": ""}

    status_color = colors.get(status, colors["reset"])

    print(f"\n{'='*60}")
    print(f"ステータス: {status_color}{status.upper()}{colors['reset']}")
    print(f"概要: {summary}")
    print(f"{'='*60}")

    if items:
        print("\n■ チェック項目:")
        for item in items:
            item_type = item.get("type", "info")
            message = item.get("message", "")
            if item_type == "ok":
                icon = "✓" if color else "[OK]"
                c = colors["ok"]
            elif item_type == "warning":
                icon = "⚠" if color else "[WARN]"
                c = colors["warning"]
            elif item_type == "error":
                icon = "✗" if color else "[ERR]"
                c = colors["error"]
            else:
                icon = "•"
                c = colors["reset"]
            print(f"  {c}{icon} {message}{colors['reset']}")

    if missing_fields:
        print(f"\n{colors['warning']}■ 未記入項目:{colors['reset']}")
        for field in missing_fields:
            field_name = field.get("field", "")
            location = field.get("location", "")
            print(f"  - {field_name} ({location})")

    print()


def main():
    parser = argparse.ArgumentParser(
        description='GEMINI書類チェッカー',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
例:
  # PDF画像をチェック
  python run_gemini_check.py --type pdf --path image.png --doc-type 暴対法誓約書 --contractor 業者名

  # スプレッドシートをチェック
  python run_gemini_check.py --type spreadsheet --spreadsheet-id XXXXX --doc-type 作業員名簿 --contractor 業者名
        """
    )

    parser.add_argument(
        '--type', '-t',
        required=True,
        choices=['pdf', 'spreadsheet'],
        help='チェック対象タイプ（pdf または spreadsheet）'
    )
    parser.add_argument(
        '--path', '-p',
        nargs='+',
        help='PDF画像のパス（複数指定可能）'
    )
    parser.add_argument(
        '--spreadsheet-id', '-s',
        help='スプレッドシートID'
    )
    parser.add_argument(
        '--sheet-name',
        help='シート名（省略時は最初のシート）'
    )
    parser.add_argument(
        '--doc-type', '-d',
        required=True,
        choices=DOC_TYPES,
        help=f'書類タイプ: {", ".join(DOC_TYPES)}'
    )
    parser.add_argument(
        '--contractor', '-c',
        required=True,
        help='業者名'
    )
    parser.add_argument(
        '--output', '-o',
        help='出力JSONファイルのパス'
    )
    parser.add_argument(
        '--json',
        action='store_true',
        help='結果をJSON形式で出力（デフォルトは整形表示）'
    )
    parser.add_argument(
        '--no-color',
        action='store_true',
        help='カラー出力を無効化'
    )

    args = parser.parse_args()

    try:
        if args.type == 'pdf':
            if not args.path:
                print("エラー: --path が必要です", file=sys.stderr)
                sys.exit(1)

            paths = [Path(p) for p in args.path]

            # パスの存在確認
            for p in paths:
                if not p.exists():
                    print(f"エラー: ファイルが見つかりません: {p}", file=sys.stderr)
                    sys.exit(1)

            if len(paths) == 1:
                result = check_pdf_image(
                    paths[0],
                    args.doc_type,
                    args.contractor
                )
            else:
                result = check_multiple_pages(
                    paths,
                    args.doc_type,
                    args.contractor
                )
        else:
            if not args.spreadsheet_id:
                print("エラー: --spreadsheet-id が必要です", file=sys.stderr)
                sys.exit(1)

            result = check_spreadsheet(
                args.spreadsheet_id,
                args.doc_type,
                args.contractor,
                args.sheet_name
            )

        # 出力
        if args.output:
            output_json = json.dumps(result, ensure_ascii=False, indent=2)
            Path(args.output).write_text(output_json, encoding='utf-8')
            print(f"結果を保存しました: {args.output}")

        if args.json:
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print_result(result, color=not args.no_color)

    except FileNotFoundError as e:
        print(f"エラー: {e}", file=sys.stderr)
        sys.exit(1)
    except ValueError as e:
        print(f"設定エラー: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"予期せぬエラー: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
