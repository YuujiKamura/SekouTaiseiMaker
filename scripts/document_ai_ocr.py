"""
Document AI OCRでPDFからテキストと座標を抽出

環境変数:
    GOOGLE_APPLICATION_CREDENTIALS: サービスアカウントJSONファイルのパス
    DOCUMENT_AI_PROJECT_ID: Google CloudプロジェクトID
    DOCUMENT_AI_LOCATION: プロセッサのロケーション (us または eu)
    DOCUMENT_AI_PROCESSOR_ID: Document AIプロセッサID
    GMAIL_TOKEN_PATH: Gmail/Drive API用トークンファイルのパス (オプション)
"""
import json
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import Optional
from google.cloud import documentai_v1 as documentai
from google.oauth2 import service_account

from config import DocumentAIConfig, GmailDriveConfig, ConfigError


# モジュールレベルの設定キャッシュ
_config: Optional[DocumentAIConfig] = None


def get_config() -> DocumentAIConfig:
    """環境変数から設定を取得（キャッシュ付き）"""
    global _config
    if _config is None:
        _config = DocumentAIConfig.from_env()
    return _config


def get_documentai_client():
    """Document AIクライアントを取得"""
    config = get_config()
    credentials = service_account.Credentials.from_service_account_file(
        str(config.credentials_path)
    )
    client = documentai.DocumentProcessorServiceClient(
        credentials=credentials,
        client_options={"api_endpoint": f"{config.location}-documentai.googleapis.com"}
    )
    return client, config


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
    client, config = get_documentai_client()

    # プロセッサ名
    processor_name = client.processor_path(
        config.project_id,
        config.location,
        config.processor_id
    )

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


def process_pdf_from_drive(file_id: str) -> dict:
    """
    Google DriveのPDFをOCR処理

    Args:
        file_id: Google DriveのファイルID

    Returns:
        process_pdfと同じ形式

    環境変数:
        GMAIL_TOKEN_PATH: Gmail/Drive API用トークンファイルのパス
    """
    from google.oauth2.credentials import Credentials
    from google.auth.transport.requests import Request
    import googleapiclient.discovery

    # 環境変数からトークンパスを取得
    drive_config = GmailDriveConfig.from_env()
    creds = Credentials.from_authorized_user_file(str(drive_config.token_path))

    if creds.expired and creds.refresh_token:
        creds.refresh(Request())

    drive_service = googleapiclient.discovery.build('drive', 'v3', credentials=creds)

    # PDFダウンロード
    request = drive_service.files().get_media(fileId=file_id)
    pdf_content = request.execute()

    # 安全な一時ファイルを使用
    with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as temp_file:
        temp_pdf = Path(temp_file.name)
        temp_file.write(pdf_content)

    try:
        result = process_pdf(temp_pdf)
    finally:
        temp_pdf.unlink()  # 削除

    return result


def extract_file_id(url: str) -> Optional[str]:
    """URLからGoogle DriveファイルIDを抽出"""
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
