# オンラインClaudeへの指示書の使い方

## 概要

6つのタスクに分割しました。それぞれ独立して実行可能です。

## タスク一覧

| タスク | ファイル | 内容 | 言語 | 依存関係 |
|--------|----------|------|------|----------|
| Task A | `task_a_pdf_viewer.md` | PDFビューワUI | Rust | なし |
| Task B | `task_b_spreadsheet_handler.md` | スプレッドシートビューワUI | Rust | なし |
| Task C | `task_c_dashboard_integration.md` | ダッシュボード連携 | Rust | A, B完了後 |
| Task D | `task_d_gemini_api.md` | GEMINI APIチェッカー | Python | なし |
| Task E | `task_e_document_ai_ocr.md` | Document AI OCR実行 | Python | なし |
| Task F | `task_f_pdf_editor.md` | PDF書き込み・出力 | Python | E完了後推奨 |

## 実行順序

```
    Task A (PDFビューワ)
           ↘
             → Task C (統合)
           ↗
    Task B (SSビューワ)

    Task D (GEMINI)  ← Python系は
    Task E (OCR)     ← 独立して
    Task F (PDF編集) ← 並行実行可能
```

## オンラインClaudeへの渡し方

### Step 1: コンテキストを渡す

まず、以下のファイルをClaudeに読ませる:

1. `src/main.rs` - 現在のコード
2. `style.css` - 現在のスタイル
3. `Cargo.toml` - 依存関係

### Step 2: タスク指示書を渡す

```
以下の指示書に従って実装してください。
コードは完全な形で出力してください。

---
[task_a_pdf_viewer.md の内容をペースト]
---
```

### Step 3: 出力を確認

Claudeが出力したコードを:
1. `src/main.rs` に追加/修正
2. `style.css` に追加
3. `trunk build` でビルド確認

## 各タスクの推定時間

- Task A: 15-20分
- Task B: 10-15分
- Task C: 15-20分
- Task D: 20-30分

## コピペ用プロンプト

### Task A を依頼する場合

```
Rust/Leptosで書かれた施工体制管理アプリがあります。
PDFビューワコンポーネントを追加してください。

現在のコードは以下です:
[main.rsをペースト]

現在のCSSは以下です:
[style.cssをペースト]

以下の仕様に従って実装してください:
[task_a_pdf_viewer.md をペースト]
```

### Task D を依頼する場合

```
PythonでGEMINI APIを使った書類チェッカーを作成してください。
建設業の書類（PDF/スプレッドシート）をチェックします。

以下の仕様に従って実装してください:
[task_d_gemini_api.md をペースト]

認証情報のパス:
- Gmail Token: C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\gmail_token.json
- GEMINI APIキー: C:\Users\yuuji\Sanyuu2Kouku\cursor_tools\summarygenerator\credentials\gemini_api_key.txt
```

## 注意事項

1. **コンテキスト長**: main.rsが長い場合、関連部分だけ抜粋
2. **エラー修正**: ビルドエラーが出たら、エラーメッセージをClaudeに渡して修正依頼
3. **統合作業**: Task Cは最後に実行。A, Bの出力を確認してから依頼

## ファイル場所

```
C:\Users\yuuji\Sanyuu2Kouku\SekouTaiseiMaker\docs\
├── implementation_plan.md      # 全体計画
├── task_a_pdf_viewer.md        # Task A 指示書
├── task_b_spreadsheet_handler.md  # Task B 指示書
├── task_c_dashboard_integration.md  # Task C 指示書
├── task_d_gemini_api.md        # Task D 指示書
└── how_to_use_instructions.md  # この説明書
```
