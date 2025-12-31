# Fix F: Gemini API 定数の整理

## 問題
1. `'gemini-2.0-flash-exp'` がハードコードされており、モデル変更時に複数ファイルを修正する必要がある
2. `DOC_TYPES` リストが `PROMPTS` の keys と重複しており、書類タイプ追加時に2箇所を更新する必要がある

## 修正ファイル
- `scripts/gemini_checker.py`
- `scripts/gemini_server.py`
- `scripts/run_gemini_check.py`
- `scripts/check_document_dates.py`
- `scripts/document_prompts.py`

## 修正1: document_prompts.py - DOC_TYPESをエクスポート

```python
# ファイル末尾に追加
# サポートする書類タイプ一覧（PROMPTSから自動生成）
DOC_TYPES = list(PROMPTS.keys())
```

## 修正2: gemini_checker.py - モデル名を定数化

```python
# 現在のコード
def init_gemini():
    """GEMINI APIを初期化"""
    api_key = get_api_key()
    genai.configure(api_key=api_key)
    return genai.GenerativeModel('gemini-2.0-flash-exp')

# 修正後
# モジュール定数（ファイル先頭、import後に追加）
GEMINI_MODEL_NAME = 'gemini-2.0-flash-exp'

def init_gemini():
    """GEMINI APIを初期化"""
    api_key = get_api_key()
    genai.configure(api_key=api_key)
    return genai.GenerativeModel(GEMINI_MODEL_NAME)
```

## 修正3: gemini_server.py - DOC_TYPESをimportに変更

```python
# 現在のコード
from gemini_checker import check_pdf_image, check_spreadsheet, check_multiple_pages

# サポートする書類タイプ
DOC_TYPES = [
    "暴対法誓約書",
    "作業員名簿",
    "下請負契約書",
    "施工体制台帳",
    "再下請負通知書",
]

# 修正後
from gemini_checker import check_pdf_image, check_spreadsheet, check_multiple_pages
from document_prompts import DOC_TYPES  # 動的に生成されたリストを使用
```

## 修正4: run_gemini_check.py - 同様にimportに変更

```python
# 現在のコード（39行目付近）
DOC_TYPES = [
    "暴対法誓約書",
    ...
]

# 修正後
from document_prompts import DOC_TYPES
```

## 修正5: check_document_dates.py - モデル名を定数参照

```python
# 現在のコード（78行目付近）
url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp:generateContent?key={GEMINI_API_KEY}"

# 修正後（ファイル先頭でimport）
from gemini_checker import GEMINI_MODEL_NAME

# 使用箇所
url = f"https://generativelanguage.googleapis.com/v1beta/models/{GEMINI_MODEL_NAME}:generateContent?key={GEMINI_API_KEY}"
```

## テスト方法
```bash
cd scripts

# インポートエラーがないことを確認
python -c "from document_prompts import DOC_TYPES; print(DOC_TYPES)"
python -c "from gemini_checker import GEMINI_MODEL_NAME; print(GEMINI_MODEL_NAME)"

# サーバー起動確認
python gemini_server.py --help
```

## 依存関係
- 単独で実行可能
- 他のFix（A〜E）とは独立
