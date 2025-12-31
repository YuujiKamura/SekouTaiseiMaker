#!/usr/bin/env python3
"""
GEMINI書類チェック用ローカルHTTPサーバー

Rust/WASMからの呼び出し用APIサーバー

使用方法:
    python gemini_server.py [--port 5000] [--host 0.0.0.0]

エンドポイント:
    POST /check/pdf
        Body: {"image_path": "path/to/image.png", "doc_type": "暴対法誓約書", "contractor": "業者名"}

    POST /check/pdf-base64
        Body: {"image_data": "base64...", "doc_type": "暴対法誓約書", "contractor": "業者名"}

    POST /check/spreadsheet
        Body: {"spreadsheet_id": "XXX", "doc_type": "作業員名簿", "contractor": "業者名"}

    GET /health
        ヘルスチェック

    GET /doc-types
        サポートする書類タイプ一覧
"""
import argparse
import logging
import sys
from pathlib import Path

# スクリプトディレクトリをパスに追加
sys.path.insert(0, str(Path(__file__).parent))

from flask import Flask, request, jsonify
from flask_cors import CORS

from document_prompts import UnknownDocTypeError, DOC_TYPES
from gemini_checker import (
    check_pdf_image,
    check_spreadsheet,
    check_multiple_pages,
    check_image_base64,
)

app = Flask(__name__)
CORS(app)  # CORS有効化（WASMからのアクセス用）
logging.basicConfig(level=logging.INFO)


@app.route('/health', methods=['GET'])
def health():
    """ヘルスチェック"""
    return jsonify({"status": "ok", "service": "gemini-checker"})


@app.route('/doc-types', methods=['GET'])
def doc_types():
    """サポートする書類タイプ一覧"""
    return jsonify({"doc_types": DOC_TYPES})


@app.route('/check/pdf', methods=['POST'])
def check_pdf():
    """
    PDF画像をチェック

    Request Body:
        {
            "image_path": "path/to/image.png",  # 単一ファイル
            "image_paths": ["path1.png", "path2.png"],  # 複数ファイル（オプション）
            "doc_type": "暴対法誓約書",
            "contractor": "業者名"
        }
    """
    try:
        data = request.json
        if not data:
            return jsonify({"error": "リクエストボディが必要です"}), 400

        doc_type = data.get('doc_type')
        contractor = data.get('contractor')

        if not doc_type or not contractor:
            return jsonify({"error": "doc_type と contractor は必須です"}), 400

        # 複数ファイルまたは単一ファイル
        image_paths = data.get('image_paths', [])
        image_path = data.get('image_path')

        if image_path and not image_paths:
            image_paths = [image_path]

        if not image_paths:
            return jsonify({"error": "image_path または image_paths が必要です"}), 400

        # パスの検証
        paths = [Path(p) for p in image_paths]
        for p in paths:
            if not p.exists():
                return jsonify({"error": f"ファイルが見つかりません: {p}"}), 404

        # チェック実行
        if len(paths) == 1:
            result = check_pdf_image(paths[0], doc_type, contractor)
        else:
            result = check_multiple_pages(paths, doc_type, contractor)

        return jsonify(result)

    except UnknownDocTypeError as e:
        app.logger.warning(f"Unknown doc type: {e}")
        return jsonify({"error": str(e), "valid_types": DOC_TYPES}), 400
    except Exception as e:
        app.logger.error(f"Check failed: {e}", exc_info=True)
        return jsonify({"error": str(e)}), 500


@app.route('/check/pdf-base64', methods=['POST'])
def check_pdf_base64():
    """
    Base64エンコードされた画像をチェック（ファイルを保存せずに処理）

    Request Body:
        {
            "image_data": "base64エンコードされた画像データ",
            "mime_type": "image/png",  # オプション、デフォルトは image/png
            "doc_type": "暴対法誓約書",
            "contractor": "業者名"
        }
    """
    try:
        data = request.json
        if not data:
            return jsonify({"error": "リクエストボディが必要です"}), 400

        image_data = data.get('image_data')
        doc_type = data.get('doc_type')
        contractor = data.get('contractor')
        mime_type = data.get('mime_type', 'image/png')

        if not image_data:
            return jsonify({"error": "image_data は必須です"}), 400
        if not doc_type or not contractor:
            return jsonify({"error": "doc_type と contractor は必須です"}), 400

        # 共通関数を使用（重複排除）
        result = check_image_base64(image_data, doc_type, contractor, mime_type)
        return jsonify(result)

    except UnknownDocTypeError as e:
        app.logger.warning(f"Unknown doc type: {e}")
        return jsonify({"error": str(e), "valid_types": DOC_TYPES}), 400
    except Exception as e:
        app.logger.error(f"Check failed: {e}", exc_info=True)
        return jsonify({"error": str(e)}), 500


@app.route('/check/spreadsheet', methods=['POST'])
def check_spreadsheet_endpoint():
    """
    スプレッドシートをチェック

    Request Body:
        {
            "spreadsheet_id": "スプレッドシートID",
            "sheet_name": "シート名",  # オプション
            "doc_type": "作業員名簿",
            "contractor": "業者名"
        }
    """
    try:
        data = request.json
        if not data:
            return jsonify({"error": "リクエストボディが必要です"}), 400

        spreadsheet_id = data.get('spreadsheet_id')
        doc_type = data.get('doc_type')
        contractor = data.get('contractor')
        sheet_name = data.get('sheet_name')

        if not spreadsheet_id:
            return jsonify({"error": "spreadsheet_id は必須です"}), 400
        if not doc_type or not contractor:
            return jsonify({"error": "doc_type と contractor は必須です"}), 400

        result = check_spreadsheet(spreadsheet_id, doc_type, contractor, sheet_name)
        return jsonify(result)

    except UnknownDocTypeError as e:
        app.logger.warning(f"Unknown doc type: {e}")
        return jsonify({"error": str(e), "valid_types": DOC_TYPES}), 400
    except Exception as e:
        app.logger.error(f"Spreadsheet check failed: {e}", exc_info=True)
        return jsonify({"error": str(e)}), 500


def main():
    parser = argparse.ArgumentParser(description='GEMINI書類チェック HTTPサーバー')
    parser.add_argument('--host', default='127.0.0.1', help='ホスト（デフォルト: 127.0.0.1）')
    parser.add_argument('--port', type=int, default=5000, help='ポート（デフォルト: 5000）')
    parser.add_argument('--debug', action='store_true', help='デバッグモード')

    args = parser.parse_args()

    print(f"GEMINI書類チェックサーバー起動中...")
    print(f"  URL: http://{args.host}:{args.port}")
    print(f"  エンドポイント:")
    print(f"    POST /check/pdf")
    print(f"    POST /check/pdf-base64")
    print(f"    POST /check/spreadsheet")
    print(f"    GET  /health")
    print(f"    GET  /doc-types")

    app.run(host=args.host, port=args.port, debug=args.debug)


if __name__ == '__main__':
    main()
