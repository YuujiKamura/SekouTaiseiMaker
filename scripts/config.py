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
        credentials_path = None
        if os.environ.get("GOOGLE_APPLICATION_CREDENTIALS"):
            credentials_path = get_path_env("GOOGLE_APPLICATION_CREDENTIALS")

        return cls(
            project_id=get_required_env(
                "DOCUMENT_AI_PROJECT_ID",
                "Google Cloud プロジェクトID"
            ),
            location=get_optional_env("DOCUMENT_AI_LOCATION", "us"),
            processor_id=get_required_env(
                "DOCUMENT_AI_PROCESSOR_ID",
                "Document AI プロセッサID"
            ),
            credentials_path=credentials_path,
        )


@dataclass
class GeminiConfig:
    """Gemini API設定"""
    api_key: str
    model_name: str

    # APIキーを探すパスのリスト
    API_KEY_PATHS = [
        Path(r"C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\credentials\gemini_api_key.txt"),
        Path.home() / "credentials" / "gemini_api_key.txt",
        Path(__file__).parent.parent / "credentials" / "gemini_api_key.txt",
        Path.home() / ".gemini_api_key",
        Path(__file__).parent / "gemini_api_key.txt",
    ]

    @classmethod
    def from_env(cls) -> "GeminiConfig":
        """環境変数から設定を読み込み"""
        # 環境変数から取得
        api_key = os.environ.get("GEMINI_API_KEY")
        if api_key:
            api_key = api_key.strip()
        else:
            # ファイルから探す
            for path in cls.API_KEY_PATHS:
                if path.exists():
                    api_key = path.read_text().strip()
                    break

        if not api_key:
            raise ConfigError(
                "Gemini APIキーが見つかりません。\n"
                "設定方法:\n"
                "  1. export GEMINI_API_KEY=your_key\n"
                "  2. ~/.gemini_api_key にキーを保存\n"
                "  3. credentials/gemini_api_key.txt にキーを保存"
            )

        return cls(
            api_key=api_key,
            model_name=get_optional_env("GEMINI_MODEL", "gemini-2.0-flash-exp"),
        )


@dataclass
class GoogleAPIConfig:
    """Google API設定（Sheets API など）"""
    api_key: str

    @classmethod
    def from_env(cls) -> "GoogleAPIConfig":
        """環境変数から設定を読み込み"""
        return cls(
            api_key=get_required_env(
                "GOOGLE_API_KEY",
                "Google API キー"
            ),
        )


@dataclass
class GmailDriveConfig:
    """Gmail/Drive API設定"""
    token_path: Path

    @classmethod
    def from_env(cls) -> "GmailDriveConfig":
        """環境変数から設定を読み込み"""
        return cls(
            token_path=get_path_env(
                "GMAIL_TOKEN_PATH",
                must_exist=True
            ),
        )
