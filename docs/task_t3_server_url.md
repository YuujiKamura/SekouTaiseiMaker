# Task T3: サーバーURL対応 (Python)

## 概要
gemini_server.pyに、URLから直接PDFを取得してチェックするエンドポイントを追加。

## 修正ファイル
- `scripts/gemini_server.py`
- `scripts/gemini_checker.py`

## 修正内容

### 1. gemini_checker.py に URL取得関数を追加

```python
import requests
import tempfile
from pathlib import Path
from typing import Optional
import io

def download_file_from_url(url: str) -> tuple[bytes, str]:
    """
    URLからファイルをダウンロード

    Args:
        url: ダウンロードURL (Google DriveやHTTP URL)

    Returns:
        (ファイルバイナリ, MIMEタイプ)
    """
    # Google Drive URLの場合、ダウンロード用URLに変換
    if "drive.google.com" in url:
        # /file/d/FILE_ID/view -> export/download形式に変換
        import re
        match = re.search(r'/d/([a-zA-Z0-9_-]+)', url)
        if match:
            file_id = match.group(1)
            url = f"https://drive.google.com/uc?export=download&id={file_id}"

    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
    }

    response = requests.get(url, headers=headers, timeout=30, allow_redirects=True)
    response.raise_for_status()

    content_type = response.headers.get('Content-Type', 'application/pdf')
    return response.content, content_type


def check_document_from_url(
    url: str,
    doc_type: str,
    contractor_name: str
) -> dict:
    """
    URLから書類をダウンロードしてチェック

    Args:
        url: 書類のURL
        doc_type: 書類タイプ
        contractor_name: 業者名

    Returns:
        チェック結果 dict
    """
    # ファイルをダウンロード
    file_data, mime_type = download_file_from_url(url)

    # 一時ファイルに保存
    suffix = '.pdf' if 'pdf' in mime_type.lower() else '.png'
    with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as f:
        f.write(file_data)
        temp_path = Path(f.name)

    try:
        # 既存のチェック関数を呼び出し
        if suffix == '.pdf':
            # PDFの場合は画像に変換が必要な場合がある
            # ここでは直接チェック（gemini-2.0-flash-expはPDF対応）
            result = check_pdf_image(temp_path, doc_type, contractor_name)
        else:
            result = check_pdf_image(temp_path, doc_type, contractor_name)

        return result
    finally:
        # 一時ファイルを削除
        temp_path.unlink(missing_ok=True)
```

### 2. gemini_server.py に新エンドポイント追加

```python
from gemini_checker import (
    check_pdf_image,
    check_spreadsheet,
    check_multiple_pages,
    check_document_from_url  # 新規追加
)

@app.route('/check/url', methods=['POST'])
def check_from_url():
    """
    URLから書類を取得してチェック

    Request Body:
        {
            "url": "https://drive.google.com/file/d/xxx/view",
            "doc_type": "暴対法誓約書",
            "contractor": "業者名"
        }
    """
    try:
        data = request.json
        if not data:
            return jsonify({"error": "リクエストボディが必要です"}), 400

        url = data.get('url')
        doc_type = data.get('doc_type')
        contractor = data.get('contractor')

        if not url:
            return jsonify({"error": "url は必須です"}), 400
        if not doc_type:
            return jsonify({"error": "doc_type は必須です"}), 400
        if not contractor:
            return jsonify({"error": "contractor は必須です"}), 400
        if doc_type not in DOC_TYPES:
            return jsonify({
                "error": f"無効な書類タイプ: {doc_type}",
                "valid_types": DOC_TYPES
            }), 400

        # URLからチェック実行
        result = check_document_from_url(url, doc_type, contractor)
        return jsonify(result)

    except requests.exceptions.RequestException as e:
        return jsonify({
            "error": f"URLからのダウンロード失敗: {str(e)}"
        }), 400
    except Exception as e:
        app.logger.error(f"Check from URL failed: {e}", exc_info=True)
        return jsonify({"error": str(e)}), 500


@app.route('/ocr/url', methods=['POST'])
def ocr_from_url():
    """
    URLからPDFを取得してOCR実行

    Request Body:
        {
            "url": "https://drive.google.com/file/d/xxx/view"
        }
    """
    try:
        data = request.json
        if not data:
            return jsonify({"error": "リクエストボディが必要です"}), 400

        url = data.get('url')
        if not url:
            return jsonify({"error": "url は必須です"}), 400

        # ファイルをダウンロード
        from gemini_checker import download_file_from_url
        file_data, mime_type = download_file_from_url(url)

        # 一時ファイルに保存
        import tempfile
        suffix = '.pdf' if 'pdf' in mime_type.lower() else '.png'
        with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as f:
            f.write(file_data)
            temp_path = Path(f.name)

        try:
            # Document AI OCRを呼び出し
            from document_ai_ocr import process_pdf
            result = process_pdf(temp_path)
            return jsonify(result)
        finally:
            temp_path.unlink(missing_ok=True)

    except Exception as e:
        app.logger.error(f"OCR from URL failed: {e}", exc_info=True)
        return jsonify({"error": str(e)}), 500
```

### 3. requirements.txt に requests 追加（なければ）

```
# requirements.txt に追記
requests>=2.28.0
```

## テスト方法

```bash
cd scripts

# サーバー起動
python gemini_server.py

# テストリクエスト（別ターミナル）
curl -X POST http://localhost:5000/check/url \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://drive.google.com/file/d/XXXXXX/view",
    "doc_type": "暴対法誓約書",
    "contractor": "テスト建設"
  }'

# ヘルスチェック
curl http://localhost:5000/health
```

## 依存関係
- なし（T1と並列実行可能）
