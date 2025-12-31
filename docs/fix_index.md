# Gemini Code Assist 指摘修正 - 指示書一覧

## 概要
GitHub PRに対するGemini Code Assistのレビュー指摘を修正するための指示書集。
オンラインのClaudeに各指示書を渡して並列で修正作業を実行可能。

## 指示書一覧

| ID | ファイル | 内容 | 優先度 | 言語 | 状態 |
|----|----------|------|--------|------|------|
| A | `fix_a_excel_file_handling.md` | ExcelファイルがSpreadsheetViewerに誤送信される問題 | 高 | Rust | 完了 |
| B | `fix_b_url_parsing.md` | Google Drive/Sheets URLパースの堅牢化 | 高 | Rust | 完了 |
| C | `fix_c_gemini_api_refactor.md` | GEMINI APIコードのリファクタリング | 中 | Python | 完了 |
| D | `fix_d_field_type_enum.md` | フィールドタイプの文字列→enum化 | 中 | Rust | 完了 |
| E | `fix_e_env_helper.md` | 環境変数取得の共通化 | 低 | Python | 完了 |
| **F** | `fix_f_gemini_constants.md` | **モデル名定数化 & DOC_TYPES動的生成** | 中 | Python | 未着手 |
| **G** | `fix_g_url_parsing_robust.md` | **URL解析のクエリパラメータ対応** | 高 | Rust | 未着手 |

## 実行順序

### 第1弾（A〜E）: 完了済み

### 第2弾（F〜G）: 並列実行可能
- **Fix F** (Python): モデル名定数化、DOC_TYPESの動的生成
- **Fix G** (Rust): URL解析のクエリパラメータ対応

両タスクは完全に独立しており、同時実行可能です。

## 推奨実行プラン

```
第1弾 (完了):
├── Fix A〜E: すべてマージ済み

第2弾 (並列実行可能):
├── Claude F: Fix F (Gemini constants) - Python
└── Claude G: Fix G (URL parsing robust) - Rust

最終:
└── 統合テスト & マージ
```

## 各指示書の使い方

1. 指示書のファイル内容をコピー
2. オンラインClaudeに貼り付け
3. 「この指示に従って修正してください」と依頼
4. 出力されたコードをレビュー
5. 問題なければ適用

## コンテキスト情報

各指示書には以下の情報が含まれています：
- **問題**: 何が問題か
- **修正箇所**: どのファイルのどの部分か
- **現在のコード**: 問題のあるコード例
- **修正後のコード**: 正しいコード例
- **テスト方法**: 修正が正しいか確認する方法

## プロジェクト構造（参考）

```
SekouTaiseiMaker/
├── src/
│   └── main.rs          # Rust/Leptos フロントエンド
├── scripts/
│   ├── gemini_checker.py
│   ├── gemini_server.py
│   ├── document_ai_ocr.py
│   └── document_prompts.py
├── style.css
└── docs/
    ├── fix_a_excel_file_handling.md
    ├── fix_b_url_parsing.md
    ├── fix_c_gemini_api_refactor.md
    ├── fix_d_field_type_enum.md
    ├── fix_e_env_helper.md
    └── fix_index.md (このファイル)
```
