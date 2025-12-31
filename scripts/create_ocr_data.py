"""
OCRトークンと画像パスを統合したJSONを作成
"""

import json
from pathlib import Path

SEKOU_TAISEI_PATH = Path(r"H:\マイドライブ\〇市道 南千反畑町第１号線舗装補修工事\５施工体制")
OUTPUT_DIR = Path(r"C:\Users\yuuji\Sanyuu2Kouku\SekouTaiseiMaker\data")

def main():
    # 画像インデックスを読み込み
    images_index_path = OUTPUT_DIR / "pdf_images" / "pdf_images_index.json"
    with open(images_index_path, 'r', encoding='utf-8') as f:
        images_index = json.load(f)

    # 画像パスをキーでマッピング
    image_map = {}
    for img in images_index:
        key = f"{img['contractor']}_{img['doc_type']}"
        image_map[key] = img['image_path']

    # debug_tokens_*.jsonを読み込んで統合
    ocr_documents = []

    for tokens_file in SEKOU_TAISEI_PATH.glob("debug_tokens_*.json"):
        # ファイル名から業者名と書類タイプを抽出
        # debug_tokens_ユナイト_09_暴対法誓約書.json
        name_parts = tokens_file.stem.replace("debug_tokens_", "").split("_", 1)
        if len(name_parts) >= 2:
            contractor = name_parts[0]
            doc_type = name_parts[1]
        else:
            contractor = tokens_file.stem
            doc_type = ""

        # トークンを読み込み
        with open(tokens_file, 'r', encoding='utf-8') as f:
            tokens = json.load(f)

        # 画像パスを取得
        key = f"{contractor}_{doc_type}"
        image_path = image_map.get(key, "")

        ocr_documents.append({
            "contractor": contractor,
            "doc_type": doc_type,
            "image_url": image_path,
            "tokens": tokens
        })

        print(f"追加: {contractor} - {doc_type} ({len(tokens)}トークン)")
        if image_path:
            print(f"  画像: {image_path}")

    # 統合JSONを保存
    output_path = OUTPUT_DIR / "ocr_documents.json"
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(ocr_documents, f, ensure_ascii=False, indent=2)

    print(f"\n完了: {output_path}")
    print(f"{len(ocr_documents)}ドキュメントを統合")


if __name__ == '__main__':
    main()
