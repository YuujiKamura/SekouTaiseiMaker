# Task F: PDF書き込み・出力

## 目的
OCRで検出した座標を使って、PDFの不足項目にテキストを書き込み、編集済みPDFを出力する。

## 技術スタック
- Python 3.x
- PyMuPDF (fitz) - PDF編集
- reportlab (オプション - 新規PDF生成用)

## ファイル構成

```
SekouTaiseiMaker/
├── scripts/
│   ├── pdf_editor.py          # PDF編集処理
│   ├── field_templates.py     # 書類タイプ別フィールド定義
│   └── run_pdf_edit.py        # CLIエントリポイント
```

## 実装

### 1. field_templates.py

```python
"""
書類タイプ別のフィールド定義
OCR結果と照合して未入力フィールドを検出
"""

FIELD_TEMPLATES = {
    "暴対法誓約書": {
        "fields": [
            {
                "name": "日付_年",
                "label": "年",
                "search_text": ["令和", "年"],
                "type": "text",
                "font_size": 12,
                "offset": {"x": 30, "y": 0}  # 「令和」の右側
            },
            {
                "name": "日付_月",
                "label": "月",
                "search_text": ["月"],
                "type": "text",
                "font_size": 12,
                "offset": {"x": -20, "y": 0}  # 「月」の左側
            },
            {
                "name": "日付_日",
                "label": "日",
                "search_text": ["日"],
                "type": "text",
                "font_size": 12,
                "offset": {"x": -20, "y": 0}
            },
            {
                "name": "宛先",
                "label": "宛先（様/御中）",
                "search_text": ["御中", "様", "殿"],
                "type": "text",
                "font_size": 12,
                "offset": {"x": -100, "y": 0}
            },
            {
                "name": "住所",
                "label": "住所",
                "search_text": ["住所", "所在地"],
                "type": "text",
                "font_size": 10,
                "offset": {"x": 50, "y": 0}
            },
            {
                "name": "会社名",
                "label": "会社名",
                "search_text": ["商号", "名称", "会社名"],
                "type": "text",
                "font_size": 12,
                "offset": {"x": 50, "y": 0}
            },
            {
                "name": "代表者",
                "label": "代表者氏名",
                "search_text": ["代表者", "氏名"],
                "type": "text",
                "font_size": 12,
                "offset": {"x": 50, "y": 0}
            }
        ],
        "markers": ["御", "中", "令", "和", "年", "月", "日", "殿", "様"]
    },

    "作業員名簿": {
        "fields": [
            {
                "name": "作成日",
                "label": "作成日",
                "search_text": ["作成", "年月日"],
                "type": "date",
                "font_size": 10,
                "offset": {"x": 50, "y": 0}
            }
        ],
        "markers": []
    }
}


def get_template(doc_type: str) -> dict:
    """書類タイプのテンプレートを取得"""
    return FIELD_TEMPLATES.get(doc_type, FIELD_TEMPLATES["暴対法誓約書"])


def find_field_positions(ocr_tokens: list, doc_type: str) -> list:
    """
    OCRトークンからフィールド位置を検出

    Returns:
        [
            {
                "name": "日付_年",
                "label": "年",
                "position": {"x": 100, "y": 200, "width": 50, "height": 20},
                "detected_text": "令和",
                "is_empty": True
            }
        ]
    """
    template = get_template(doc_type)
    results = []

    for field in template["fields"]:
        # 検索テキストに一致するトークンを探す
        for token in ocr_tokens:
            for search_text in field["search_text"]:
                if search_text in token["text"]:
                    # フィールド位置を計算
                    pos = token["pixels"].copy()
                    pos["x"] += field["offset"]["x"]
                    pos["y"] += field["offset"]["y"]

                    # 周辺に値があるか確認（未入力判定）
                    is_empty = check_if_empty(ocr_tokens, pos, field)

                    results.append({
                        "name": field["name"],
                        "label": field["label"],
                        "position": pos,
                        "detected_text": token["text"],
                        "is_empty": is_empty,
                        "font_size": field["font_size"]
                    })
                    break

    return results


def check_if_empty(tokens: list, position: dict, field: dict) -> bool:
    """指定位置の周辺にテキストがないか確認"""
    # 簡易実装: 位置の近くに数字や文字がなければ空と判定
    x, y = position["x"], position["y"]
    threshold = 30  # ピクセル

    for token in tokens:
        tx = token["pixels"]["x"]
        ty = token["pixels"]["y"]

        if abs(tx - x) < threshold and abs(ty - y) < threshold:
            # 近くにトークンがある
            if token["text"] not in ["　", " ", ""]:
                return False

    return True
```

### 2. pdf_editor.py

```python
"""
PDFにテキストを書き込む
"""
import json
from pathlib import Path
from typing import Optional
import fitz  # PyMuPDF

# 日本語フォント（MS Gothic等）
FONT_PATH = "C:/Windows/Fonts/msgothic.ttc"


def load_pdf(pdf_path: Path) -> fitz.Document:
    """PDFを読み込む"""
    return fitz.open(str(pdf_path))


def load_pdf_from_bytes(pdf_bytes: bytes) -> fitz.Document:
    """バイト列からPDFを読み込む"""
    return fitz.open(stream=pdf_bytes, filetype="pdf")


def add_text_to_pdf(
    doc: fitz.Document,
    page_num: int,
    x: float,
    y: float,
    text: str,
    font_size: float = 12,
    font_name: str = "japan",
    color: tuple = (0, 0, 0)
) -> None:
    """
    PDFにテキストを追加

    Args:
        doc: PyMuPDF Document
        page_num: ページ番号（0始まり）
        x, y: 座標（ポイント単位、左上原点）
        text: 書き込むテキスト
        font_size: フォントサイズ
        font_name: フォント名
        color: RGB色（0-1）
    """
    page = doc[page_num]

    # 日本語フォントを登録
    if Path(FONT_PATH).exists():
        page.insert_font(fontname="msgothic", fontfile=FONT_PATH)
        font_name = "msgothic"

    # テキスト挿入
    text_point = fitz.Point(x, y)
    page.insert_text(
        text_point,
        text,
        fontname=font_name,
        fontsize=font_size,
        color=color
    )


def fill_fields(
    pdf_path: Path,
    fields: list,
    output_path: Path,
    dpi: int = 150
) -> Path:
    """
    複数のフィールドにテキストを書き込んで保存

    Args:
        pdf_path: 入力PDFパス
        fields: フィールドリスト
            [
                {
                    "page": 0,
                    "x": 100,
                    "y": 200,
                    "text": "入力値",
                    "font_size": 12
                }
            ]
        output_path: 出力PDFパス
        dpi: 座標計算用DPI

    Returns:
        出力ファイルパス
    """
    doc = load_pdf(pdf_path)

    # DPI→ポイント変換係数
    # PDFは72dpi基準、OCR座標はdpi（通常150）基準
    scale = 72 / dpi

    for field in fields:
        page_num = field.get("page", 0)
        x = field["x"] * scale
        y = field["y"] * scale
        text = field["text"]
        font_size = field.get("font_size", 12)

        add_text_to_pdf(doc, page_num, x, y, text, font_size)

    # 保存
    doc.save(str(output_path))
    doc.close()

    return output_path


def fill_from_ocr_and_input(
    pdf_path: Path,
    ocr_result: dict,
    user_inputs: dict,
    doc_type: str,
    output_path: Path
) -> Path:
    """
    OCR結果とユーザー入力からPDFを編集

    Args:
        pdf_path: 入力PDFパス
        ocr_result: OCR結果JSON
        user_inputs: ユーザー入力 {"日付_年": "7", "日付_月": "1", ...}
        doc_type: 書類タイプ
        output_path: 出力PDFパス

    Returns:
        出力ファイルパス
    """
    from field_templates import find_field_positions

    # OCRトークンを取得
    tokens = []
    for page in ocr_result.get("pages", []):
        tokens.extend(page.get("tokens", []))

    # フィールド位置を検出
    field_positions = find_field_positions(tokens, doc_type)

    # 書き込みフィールドを作成
    fields_to_write = []
    for fp in field_positions:
        if fp["name"] in user_inputs and user_inputs[fp["name"]]:
            fields_to_write.append({
                "page": 0,
                "x": fp["position"]["x"],
                "y": fp["position"]["y"],
                "text": user_inputs[fp["name"]],
                "font_size": fp.get("font_size", 12)
            })

    # PDF編集
    return fill_fields(pdf_path, fields_to_write, output_path)


def download_and_edit_from_drive(
    file_id: str,
    user_inputs: dict,
    doc_type: str,
    output_path: Path
) -> Path:
    """
    Google DriveからPDFをダウンロードして編集

    Args:
        file_id: Google DriveファイルID
        user_inputs: ユーザー入力
        doc_type: 書類タイプ
        output_path: 出力PDFパス

    Returns:
        出力ファイルパス
    """
    from google.oauth2.credentials import Credentials
    from google.auth.transport.requests import Request
    import googleapiclient.discovery

    from document_ai_ocr import process_pdf_from_drive

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
    temp_pdf = output_path.parent / f"temp_{file_id}.pdf"
    temp_pdf.write_bytes(pdf_content)

    try:
        # OCR実行
        ocr_result = process_pdf_from_drive(file_id)

        # PDF編集
        result_path = fill_from_ocr_and_input(
            temp_pdf,
            ocr_result,
            user_inputs,
            doc_type,
            output_path
        )
    finally:
        if temp_pdf.exists():
            temp_pdf.unlink()

    return result_path
```

### 3. run_pdf_edit.py

```python
"""
CLIからPDF編集を実行
"""
import argparse
import json
import sys
from pathlib import Path

from pdf_editor import fill_fields, fill_from_ocr_and_input, download_and_edit_from_drive
from document_ai_ocr import extract_file_id


def main():
    parser = argparse.ArgumentParser(description='PDF編集ツール')

    subparsers = parser.add_subparsers(dest='command', help='コマンド')

    # simple: 座標指定で直接書き込み
    simple_parser = subparsers.add_parser('simple', help='座標指定で書き込み')
    simple_parser.add_argument('--pdf', required=True, help='入力PDFパス')
    simple_parser.add_argument('--output', '-o', required=True, help='出力PDFパス')
    simple_parser.add_argument('--fields', required=True, help='フィールドJSON')

    # auto: OCR結果から自動検出して書き込み
    auto_parser = subparsers.add_parser('auto', help='OCR自動検出で書き込み')
    auto_parser.add_argument('--pdf', help='入力PDFパス')
    auto_parser.add_argument('--url', help='Google Drive URL')
    auto_parser.add_argument('--ocr', help='OCR結果JSONパス')
    auto_parser.add_argument('--inputs', required=True, help='入力値JSON')
    auto_parser.add_argument('--doc-type', required=True, help='書類タイプ')
    auto_parser.add_argument('--output', '-o', required=True, help='出力PDFパス')

    args = parser.parse_args()

    if args.command == 'simple':
        # 座標指定モード
        fields = json.loads(Path(args.fields).read_text(encoding='utf-8'))
        result = fill_fields(
            Path(args.pdf),
            fields,
            Path(args.output)
        )
        print(f"保存: {result}")

    elif args.command == 'auto':
        # 自動検出モード
        inputs = json.loads(Path(args.inputs).read_text(encoding='utf-8'))

        if args.url:
            file_id = extract_file_id(args.url)
            result = download_and_edit_from_drive(
                file_id,
                inputs,
                args.doc_type,
                Path(args.output)
            )
        elif args.pdf:
            ocr_result = json.loads(Path(args.ocr).read_text(encoding='utf-8'))
            result = fill_from_ocr_and_input(
                Path(args.pdf),
                ocr_result,
                inputs,
                args.doc_type,
                Path(args.output)
            )
        else:
            print("--pdf または --url が必要です", file=sys.stderr)
            sys.exit(1)

        print(f"保存: {result}")

    else:
        parser.print_help()


if __name__ == '__main__':
    main()
```

## 使用例

### 座標指定で直接書き込み

```bash
# fields.json
# [{"page": 0, "x": 100, "y": 200, "text": "令和7年", "font_size": 12}]

python run_pdf_edit.py simple \
    --pdf "input.pdf" \
    --fields "fields.json" \
    --output "output.pdf"
```

### OCR自動検出で書き込み

```bash
# inputs.json
# {"日付_年": "7", "日付_月": "1", "日付_日": "15", "宛先": "熊本市長"}

python run_pdf_edit.py auto \
    --url "https://drive.google.com/file/d/xxx/view" \
    --inputs "inputs.json" \
    --doc-type "暴対法誓約書" \
    --output "edited.pdf"
```

## HTTPサーバー連携

```python
# pdf_server.py に追加
@app.route('/pdf/edit', methods=['POST'])
def edit_pdf():
    data = request.json
    """
    {
        "url": "https://drive.google.com/...",
        "doc_type": "暴対法誓約書",
        "inputs": {"日付_年": "7", ...}
    }
    """
    file_id = extract_file_id(data['url'])
    output_path = Path(f"temp/edited_{file_id}.pdf")

    result_path = download_and_edit_from_drive(
        file_id,
        data['inputs'],
        data['doc_type'],
        output_path
    )

    # ファイルを返す
    return send_file(str(result_path), mimetype='application/pdf')
```

## 必要なパッケージ

```bash
pip install pymupdf
```

## 注意事項

1. **日本語フォント**: Windowsでは `msgothic.ttc` を使用
2. **座標変換**: OCR座標（150dpi）→PDF座標（72dpi）の変換が必要
3. **フォント埋め込み**: 配布用PDFにはフォント埋め込みを検討
4. **元PDFの保護**: 編集不可PDFは処理できない
