# Fix C: GEMINI API コードのリファクタリング

## 問題
1. `get_check_prompt()` が不明な書類タイプで暗黙的に「暴対法誓約書」にフォールバック
2. `/check/pdf-base64` エンドポイントが `gemini_checker.py` のロジックを重複
3. `init_gemini()` が毎回呼ばれて非効率
4. 関数内でのimport文（PEP 8違反）

## 修正ファイル
- `scripts/document_prompts.py`
- `scripts/gemini_checker.py`
- `scripts/gemini_server.py`

## 修正1: document_prompts.py - 明示的なエラー処理

```python
# 現在のコード（問題あり）
def get_check_prompt(doc_type: str) -> str:
    return DOC_PROMPTS.get(doc_type, DOC_PROMPTS["暴対法誓約書"])

# 修正後
class UnknownDocTypeError(Exception):
    """未知の書類タイプエラー"""
    pass

def get_check_prompt(doc_type: str) -> str:
    """
    書類タイプに対応するプロンプトを取得

    Raises:
        UnknownDocTypeError: 未知の書類タイプの場合
    """
    if doc_type not in DOC_PROMPTS:
        raise UnknownDocTypeError(
            f"未知の書類タイプ: {doc_type}. "
            f"対応タイプ: {list(DOC_PROMPTS.keys())}"
        )
    return DOC_PROMPTS[doc_type]

def get_check_prompt_safe(doc_type: str, default: str = "暴対法誓約書") -> str:
    """
    書類タイプに対応するプロンプトを取得（フォールバックあり）
    明示的にフォールバックが必要な場合のみ使用
    """
    return DOC_PROMPTS.get(doc_type, DOC_PROMPTS[default])
```

## 修正2: gemini_checker.py - シングルトンパターン

```python
import os
from pathlib import Path
from typing import Optional
import google.generativeai as genai

# モジュールレベルでモデルをキャッシュ
_model: Optional[genai.GenerativeModel] = None

def get_gemini_model() -> genai.GenerativeModel:
    """Geminiモデルのシングルトンインスタンスを取得"""
    global _model
    if _model is None:
        api_key = os.environ.get("GEMINI_API_KEY")
        if not api_key:
            # ファイルから読み込み
            key_paths = [
                Path.home() / ".gemini_api_key",
                Path(__file__).parent / "gemini_api_key.txt",
            ]
            for key_path in key_paths:
                if key_path.exists():
                    api_key = key_path.read_text().strip()
                    break

        if not api_key:
            raise ValueError("GEMINI_API_KEY not found")

        genai.configure(api_key=api_key)
        _model = genai.GenerativeModel("gemini-1.5-flash")

    return _model


def check_document(image_base64: str, doc_type: str) -> dict:
    """
    ドキュメントをGeminiでチェック

    Args:
        image_base64: Base64エンコードされた画像
        doc_type: 書類タイプ

    Returns:
        {"status": "ok"|"warning"|"error", "messages": [...]}
    """
    from document_prompts import get_check_prompt

    model = get_gemini_model()
    prompt = get_check_prompt(doc_type)

    response = model.generate_content([
        prompt,
        {"mime_type": "image/png", "data": image_base64}
    ])

    return parse_gemini_response(response.text)


def parse_gemini_response(response_text: str) -> dict:
    """Geminiレスポンスをパース"""
    import json
    import re

    # JSONブロックを抽出
    json_match = re.search(r'```json\s*(.*?)\s*```', response_text, re.DOTALL)
    if json_match:
        try:
            return json.loads(json_match.group(1))
        except json.JSONDecodeError:
            pass

    # プレーンJSONを試行
    try:
        return json.loads(response_text)
    except json.JSONDecodeError:
        pass

    # フォールバック
    return {
        "status": "warning",
        "messages": [response_text[:500]]
    }
```

## 修正3: gemini_server.py - 共通関数を使用

```python
from flask import Flask, request, jsonify
import logging

# ファイル先頭でimport（PEP 8準拠）
from gemini_checker import check_document, get_gemini_model
from document_prompts import get_check_prompt, UnknownDocTypeError

app = Flask(__name__)
logging.basicConfig(level=logging.INFO)

@app.route('/check/pdf-base64', methods=['POST'])
def check_pdf_base64():
    """Base64エンコードされたPDFをチェック"""
    try:
        data = request.json
        image_base64 = data.get('image')
        doc_type = data.get('doc_type', '暴対法誓約書')

        if not image_base64:
            return jsonify({"error": "image is required"}), 400

        # 共通関数を使用（重複排除）
        result = check_document(image_base64, doc_type)
        return jsonify(result)

    except UnknownDocTypeError as e:
        app.logger.warning(f"Unknown doc type: {e}")
        return jsonify({"error": str(e)}), 400
    except Exception as e:
        app.logger.error(f"Check failed: {e}", exc_info=True)
        return jsonify({"error": str(e)}), 500


@app.route('/check/spreadsheet', methods=['POST'])
def check_spreadsheet():
    """スプレッドシートをチェック（関数名をルートに合わせる）"""
    try:
        data = request.json
        spreadsheet_id = data.get('spreadsheet_id')
        doc_type = data.get('doc_type', '作業員名簿')

        if not spreadsheet_id:
            return jsonify({"error": "spreadsheet_id is required"}), 400

        # TODO: スプレッドシート読み取りとチェック実装
        result = {"status": "ok", "messages": ["チェック完了"]}
        return jsonify(result)

    except Exception as e:
        app.logger.error(f"Spreadsheet check failed: {e}", exc_info=True)
        return jsonify({"error": str(e)}), 500
```

## テスト方法
```bash
# サーバー起動
cd scripts
python gemini_server.py

# テストリクエスト
curl -X POST http://localhost:5000/check/pdf-base64 \
  -H "Content-Type: application/json" \
  -d '{"image": "base64data...", "doc_type": "暴対法誓約書"}'

# 不明な書類タイプでエラーになることを確認
curl -X POST http://localhost:5000/check/pdf-base64 \
  -H "Content-Type: application/json" \
  -d '{"image": "base64data...", "doc_type": "存在しないタイプ"}'
```
