"""
CLIからOCRを実行
"""
import argparse
import json
import sys
from pathlib import Path

from document_ai_ocr import process_pdf, process_pdf_from_drive, extract_file_id


def main():
    parser = argparse.ArgumentParser(description='Document AI OCR')
    parser.add_argument('--pdf', help='ローカルPDFファイルのパス')
    parser.add_argument('--url', help='Google Drive URL')
    parser.add_argument('--file-id', help='Google DriveファイルID')
    parser.add_argument('--output', '-o', help='出力JSONファイル')
    parser.add_argument('--pretty', action='store_true', help='整形出力')

    args = parser.parse_args()

    # 入力ソース判定
    if args.pdf:
        result = process_pdf(Path(args.pdf))
    elif args.url:
        file_id = extract_file_id(args.url)
        if not file_id:
            print(f"URLからファイルIDを抽出できません: {args.url}", file=sys.stderr)
            sys.exit(1)
        result = process_pdf_from_drive(file_id)
    elif args.file_id:
        result = process_pdf_from_drive(args.file_id)
    else:
        print("--pdf, --url, --file-id のいずれかを指定してください", file=sys.stderr)
        sys.exit(1)

    # 出力
    indent = 2 if args.pretty else None
    output_text = json.dumps(result, ensure_ascii=False, indent=indent)

    if args.output:
        Path(args.output).write_text(output_text, encoding='utf-8')
        print(f"保存: {args.output}")
    else:
        print(output_text)


if __name__ == '__main__':
    main()
