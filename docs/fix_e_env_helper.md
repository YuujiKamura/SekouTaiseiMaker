# Fix E: 環境変数取得の共通化

## 問題
`scripts/document_ai_ocr.py` で環境変数の取得とバリデーションが繰り返されており、重複コードになっている。

## 修正ファイル
- `scripts/document_ai_ocr.py`
- （新規）`scripts/config.py`

## 修正1: config.py - 共通設定モジュールの作成

```python
"""
共通設定モジュール
環境変数の取得とバリデーションを一元管理
"""
import os
from pathlib import Path
from typing import Optional
from dataclasses import dataclass


class ConfigError(Exception):
    """設定エラー"""
    pass


def get_required_env(name: str, description: str = "") -> str:
    """
    必須環境変数を取得

    Args:
        name: 環境変数名
        description: エラーメッセージ用の説明

    Returns:
        環境変数の値

    Raises:
        ConfigError: 環境変数が設定されていない場合
    """
    value = os.environ.get(name)
    if not value:
        desc = f" ({description})" if description else ""
        raise ConfigError(
            f"環境変数 {name}{desc} が設定されていません。\n"
            f"設定例: export {name}=your_value"
        )
    return value


def get_optional_env(name: str, default: str = "") -> str:
    """
    オプション環境変数を取得

    Args:
        name: 環境変数名
        default: デフォルト値

    Returns:
        環境変数の値またはデフォルト値
    """
    return os.environ.get(name, default)


def get_path_env(name: str, must_exist: bool = True) -> Path:
    """
    パス型の環境変数を取得

    Args:
        name: 環境変数名
        must_exist: パスが存在することを要求するか

    Returns:
        Pathオブジェクト

    Raises:
        ConfigError: 環境変数が未設定またはパスが存在しない場合
    """
    value = get_required_env(name)
    path = Path(value)
    if must_exist and not path.exists():
        raise ConfigError(f"{name} のパス {path} が存在しません")
    return path


@dataclass
class DocumentAIConfig:
    """Document AI設定"""
    project_id: str
    location: str
    processor_id: str
    credentials_path: Optional[Path]

    @classmethod
    def from_env(cls) -> "DocumentAIConfig":
        """環境変数から設定を読み込み"""
        return cls(
            project_id=get_required_env(
                "GOOGLE_CLOUD_PROJECT",
                "Google Cloud プロジェクトID"
            ),
            location=get_optional_env("DOCUMENT_AI_LOCATION", "us"),
            processor_id=get_required_env(
                "DOCUMENT_AI_PROCESSOR_ID",
                "Document AI プロセッサID"
            ),
            credentials_path=get_path_env("GOOGLE_APPLICATION_CREDENTIALS")
            if os.environ.get("GOOGLE_APPLICATION_CREDENTIALS")
            else None,
        )


@dataclass
class GeminiConfig:
    """Gemini API設定"""
    api_key: str
    model_name: str

    @classmethod
    def from_env(cls) -> "GeminiConfig":
        """環境変数から設定を読み込み"""
        # APIキーを複数の場所から探す
        api_key = os.environ.get("GEMINI_API_KEY")
        if not api_key:
            key_paths = [
                Path.home() / ".gemini_api_key",
                Path(__file__).parent / "gemini_api_key.txt",
            ]
            for path in key_paths:
                if path.exists():
                    api_key = path.read_text().strip()
                    break

        if not api_key:
            raise ConfigError(
                "Gemini APIキーが見つかりません。\n"
                "設定方法:\n"
                "  1. export GEMINI_API_KEY=your_key\n"
                "  2. ~/.gemini_api_key にキーを保存\n"
                "  3. scripts/gemini_api_key.txt にキーを保存"
            )

        return cls(
            api_key=api_key,
            model_name=get_optional_env("GEMINI_MODEL", "gemini-1.5-flash"),
        )
```

## 修正2: document_ai_ocr.py での使用

```python
# 現在のコード（重複あり）
PROJECT_ID = os.environ.get("GOOGLE_CLOUD_PROJECT")
if not PROJECT_ID:
    raise ValueError("GOOGLE_CLOUD_PROJECT environment variable is required")

LOCATION = os.environ.get("DOCUMENT_AI_LOCATION", "us")

PROCESSOR_ID = os.environ.get("DOCUMENT_AI_PROCESSOR_ID")
if not PROCESSOR_ID:
    raise ValueError("DOCUMENT_AI_PROCESSOR_ID environment variable is required")

# 修正後
from config import DocumentAIConfig, ConfigError

try:
    config = DocumentAIConfig.from_env()
except ConfigError as e:
    print(f"設定エラー: {e}")
    sys.exit(1)

# 使用例
def get_documentai_client():
    return documentai.DocumentProcessorServiceClient(
        client_options={"api_endpoint": f"{config.location}-documentai.googleapis.com"}
    )
```

## 修正3: gemini_checker.py での使用

```python
# 現在のコード
api_key = os.environ.get("GEMINI_API_KEY")
if not api_key:
    # ファイルから読み込み...

# 修正後
from config import GeminiConfig, ConfigError

try:
    gemini_config = GeminiConfig.from_env()
except ConfigError as e:
    print(f"Gemini設定エラー: {e}")
    sys.exit(1)

genai.configure(api_key=gemini_config.api_key)
model = genai.GenerativeModel(gemini_config.model_name)
```

## テスト方法
```bash
# 環境変数が未設定の場合のエラーメッセージを確認
unset GOOGLE_CLOUD_PROJECT
python -c "from config import DocumentAIConfig; DocumentAIConfig.from_env()"
# → ConfigError: 環境変数 GOOGLE_CLOUD_PROJECT (Google Cloud プロジェクトID) が設定されていません。

# 正常な設定で動作確認
export GOOGLE_CLOUD_PROJECT=my-project
export DOCUMENT_AI_PROCESSOR_ID=abc123
python document_ai_ocr.py --help
```
