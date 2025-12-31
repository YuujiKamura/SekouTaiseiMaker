# Task E: Document AI OCR実行スクリプト

## 目的
Google Document AI を使ってPDFからテキストと座標を抽出するスクリプトを作成する。

## 技術スタック
- Python 3.x
- google-cloud-documentai
- PyMuPDF (PDF→画像変換)

## 既存の認証情報
```
プロジェクトID: visionapi-437405
ロケーション: us (またはeu)
プロセッサID: 既存のOCRプロセッサを使用
認証ファイル: C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\credentials\visionapi-437405-734d18d13418.json
```

## ファイル構成

```
SekouTaiseiMaker/
├── scripts/
│   ├── document_ai_ocr.py     # メインOCR処理
│   └── run_ocr.py             # CLIエントリポイント
```

## 実装

### 1. document_ai_ocr.py

```python
"""
Document AI OCRでPDFからテキストと座標を抽出
"""
import json
from pathlib import Path
from typing import Optional
from google.cloud import documentai_v1 as documentai
from google.oauth2 import service_account

# 設定
CREDENTIALS_PATH = Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\credentials\visionapi-437405-734d18d13418.json")
PROJECT_ID = "visionapi-437405"
LOCATION = "us"  # または "eu"
PROCESSOR_ID = "YOUR_PROCESSOR_ID"  # Document AI コンソールで確認

def get_documentai_client():
    """Document AIクライアントを取得"""
    credentials = service_account.Credentials.from_service_account_file(
        str(CREDENTIALS_PATH)
    )
    client = documentai.DocumentProcessorServiceClient(
        credentials=credentials,
        client_options={"api_endpoint": f"{LOCATION}-documentai.googleapis.com"}
    )
    return client


def process_pdf(pdf_path: Path) -> dict:
    """
    PDFをOCR処理してトークンリストを返す

    Args:
        pdf_path: PDFファイルのパス

    Returns:
        {
            "page_count": 1,
            "pages": [
                {
                    "page_number": 1,
                    "width": 1681,
                    "height": 2378,
                    "tokens": [
                        {
                            "text": "御中",
                            "confidence": 0.98,
                            "normalized": {
                                "x": 0.42,
                                "y": 0.23,
                                "width": 0.05,
                                "height": 0.02
                            },
                            "pixels": {
                                "x": 708,
                                "y": 541,
                                "width": 82,
                                "height": 39
                            }
                        }
                    ]
                }
            ]
        }
    """
    client = get_documentai_client()

    # プロセッサ名
    processor_name = client.processor_path(PROJECT_ID, LOCATION, PROCESSOR_ID)

    # PDFを読み込み
    with open(pdf_path, "rb") as f:
        pdf_content = f.read()

    # リクエスト作成
    raw_document = documentai.RawDocument(
        content=pdf_content,
        mime_type="application/pdf"
    )
    request = documentai.ProcessRequest(
        name=processor_name,
        raw_document=raw_document
    )

    # OCR実行
    result = client.process_document(request=request)
    document = result.document

    # 結果を整形
    output = {
        "page_count": len(document.pages),
        "pages": []
    }

    for page in document.pages:
        page_width = page.dimension.width
        page_height = page.dimension.height

        page_data = {
            "page_number": page.page_number,
            "width": page_width,
            "height": page_height,
            "tokens": []
        }

        # トークン（単語単位）を抽出
        for token in page.tokens:
            # バウンディングボックス取得
            vertices = token.layout.bounding_poly.normalized_vertices
            if len(vertices) >= 4:
                x_coords = [v.x for v in vertices]
                y_coords = [v.y for v in vertices]

                min_x = min(x_coords)
                min_y = min(y_coords)
                max_x = max(x_coords)
                max_y = max(y_coords)

                # テキスト取得
                text = get_text_from_layout(token.layout, document.text)

                token_data = {
                    "text": text,
                    "confidence": token.layout.confidence,
                    "normalized": {
                        "x": min_x,
                        "y": min_y,
                        "width": max_x - min_x,
                        "height": max_y - min_y
                    },
                    "pixels": {
                        "x": int(min_x * page_width),
                        "y": int(min_y * page_height),
                        "width": int((max_x - min_x) * page_width),
                        "height": int((max_y - min_y) * page_height)
                    }
                }
                page_data["tokens"].append(token_data)

        output["pages"].append(page_data)

    return output


def get_text_from_layout(layout, full_text: str) -> str:
    """レイアウトからテキストを抽出"""
    text = ""
    for segment in layout.text_anchor.text_segments:
        start = int(segment.start_index) if segment.start_index else 0
        end = int(segment.end_index)
        text += full_text[start:end]
    return text.strip()


def process_pdf_from_drive(file_id: str, temp_dir: Path = None) -> dict:
    """
    Google DriveのPDFをOCR処理

    Args:
        file_id: Google DriveのファイルID
        temp_dir: 一時ファイル保存先

    Returns:
        process_pdfと同じ形式
    """
    from google.oauth2.credentials import Credentials
    from google.auth.transport.requests import Request
    import googleapiclient.discovery

    # Gmail tokenでDrive API認証
    token_path = Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json")
    creds = Credentials.from_authorized_user_file(str(token_path))

    if creds.expired and creds.refresh_token:
        creds.refresh(Request())

    drive_service = googleapiclient.discovery.build('drive', 'v3', credentials=creds)

    # PDFダウンロード
    request = drive_service.files().get_media(fileId=file_id)
    pdf_content = request.execute()

    # 一時ファイルに保存
    if temp_dir is None:
        temp_dir = Path.cwd() / "temp"
    temp_dir.mkdir(exist_ok=True)

    temp_pdf = temp_dir / f"{file_id}.pdf"
    temp_pdf.write_bytes(pdf_content)

    try:
        result = process_pdf(temp_pdf)
    finally:
        temp_pdf.unlink()  # 削除

    return result


def extract_file_id(url: str) -> Optional[str]:
    """URLからGoogle DriveファイルIDを抽出"""
    import re
    patterns = [
        r'/file/d/([a-zA-Z0-9-_]+)',
        r'/d/([a-zA-Z0-9-_]+)',
        r'id=([a-zA-Z0-9-_]+)',
    ]
    for pattern in patterns:
        match = re.search(pattern, url)
        if match:
            return match.group(1)
    return None
```

### 2. run_ocr.py

```python
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
```

## 使用例

```bash
# ローカルPDFをOCR
python run_ocr.py --pdf "path/to/document.pdf" -o result.json --pretty

# Google DriveのPDFをOCR
python run_ocr.py --url "https://drive.google.com/file/d/xxx/view" -o result.json

# ファイルIDを直接指定
python run_ocr.py --file-id "1abc123xyz" -o result.json
```

## 出力JSON形式

```json
{
  "page_count": 1,
  "pages": [
    {
      "page_number": 1,
      "width": 1681,
      "height": 2378,
      "tokens": [
        {
          "text": "御中",
          "confidence": 0.98,
          "normalized": {
            "x": 0.42,
            "y": 0.23,
            "width": 0.05,
            "height": 0.02
          },
          "pixels": {
            "x": 708,
            "y": 541,
            "width": 82,
            "height": 39
          }
        }
      ]
    }
  ]
}
```

## Rust側との連携

Webアプリから呼び出す場合、HTTPサーバー経由で:

```python
# ocr_server.py に追加
@app.route('/ocr', methods=['POST'])
def run_ocr():
    data = request.json
    if 'url' in data:
        file_id = extract_file_id(data['url'])
        result = process_pdf_from_drive(file_id)
    elif 'file_id' in data:
        result = process_pdf_from_drive(data['file_id'])
    else:
        return jsonify({"error": "url or file_id required"}), 400

    return jsonify(result)
```

## 必要なパッケージ

```bash
pip install google-cloud-documentai google-auth google-api-python-client
```

## 注意事項

1. **プロセッサID**: Document AIコンソールで確認して設定
2. **料金**: Document AIは1000ページ/月まで無料、それ以降は課金
3. **ファイルサイズ**: 20MB以下のPDFのみ対応
