# 書類ビューワ機能 実装計画

## 概要
各社の書類をクリックしたら、書類タイプに応じたビューワが開く機能を追加する。

## 機能一覧

### Rust/WASM (フロントエンド)

| Task | 内容 |
|------|------|
| Task A | PDFビューワUI - プレビュー、OCRボタン、入力フォーム |
| Task B | スプレッドシートビューワUI - 開くボタン、チェック結果表示 |
| Task C | ダッシュボード連携 - クリックでビューワ起動 |

### Python (バックエンド/CLI)

| Task | 内容 |
|------|------|
| Task D | GEMINI APIチェッカー - PDF/SS内容の自動確認 |
| Task E | Document AI OCR - テキスト座標抽出 |
| Task F | PDF編集 - 不足項目書き込み・出力 |

## 依存関係

```
Rust系:
  Task A ─┬─→ Task C (統合)
  Task B ─┘

Python系:
  Task D (GEMINI) ← 独立
  Task E (OCR) ← 独立
  Task F (PDF編集) ← Task E の出力を使用
```

## 推奨実行順序

### 並行実行グループ1
- Task A (PDFビューワUI)
- Task B (スプレッドシートUI)
- Task D (GEMINI API)
- Task E (Document AI OCR)

### 並行実行グループ2
- Task C (ダッシュボード統合) ← A, B完了後
- Task F (PDF編集) ← E完了後

## ファイル一覧

```
docs/
├── implementation_plan.md        # この計画書
├── how_to_use_instructions.md    # 使い方説明
├── task_a_pdf_viewer.md          # Rust: PDFビューワ
├── task_b_spreadsheet_handler.md # Rust: SSビューワ
├── task_c_dashboard_integration.md # Rust: 統合
├── task_d_gemini_api.md          # Python: GEMINI
├── task_e_document_ai_ocr.md     # Python: OCR
└── task_f_pdf_editor.md          # Python: PDF編集
```
