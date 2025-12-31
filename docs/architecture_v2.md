# SekouTaiseiMaker v2 アーキテクチャ設計書

## 目標

ダッシュボードから書類を開き、OCR/AIチェックを実行し、結果をリストに反映する一連のフローを実現する。

## 現状の問題

```
[ダッシュボード] ──クリック──> [PdfViewer] ← iframe表示のみ、行き止まり
                               ↓
                          (何もできない)

[OcrViewer] ← メニューからのみ、ダッシュボードと非連携
[gemini_server.py] ← WASMから呼ばれていない
```

## 目標アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                    WASM (Rust/Leptos)                           │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐    │
│  │  Dashboard   │────>│  DocViewer   │────>│ ResultPanel  │    │
│  │              │<────│  (PDF/Sheet) │<────│              │    │
│  │  ・書類リスト │     │  ・プレビュー │     │  ・OCR結果   │    │
│  │  ・所見バッジ │     │  ・OCRボタン  │     │  ・チェック結果│   │
│  │  ・進捗表示  │     │  ・チェックBtn│     │  ・編集フォーム│   │
│  └──────────────┘     └──────┬───────┘     └──────────────┘    │
│                              │ fetch                            │
└──────────────────────────────┼──────────────────────────────────┘
                               │ HTTP (localhost:5000)
┌──────────────────────────────┼──────────────────────────────────┐
│                    Python Backend                               │
│  ┌──────────────┐     ┌──────┴───────┐     ┌──────────────┐    │
│  │ gemini_server│────>│  Dispatcher  │────>│ document_ai  │    │
│  │  Flask+CORS  │     │              │     │    _ocr.py   │    │
│  │              │     │  /check/*    │     └──────────────┘    │
│  │              │     │  /ocr/*      │     ┌──────────────┐    │
│  │              │     │              │────>│gemini_checker│    │
│  └──────────────┘     └──────────────┘     │    .py       │    │
│                                             └──────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## データ構造の拡張

### 現在の DocStatus
```rust
pub struct DocStatus {
    pub status: bool,              // 完了フラグ
    pub file: Option<String>,      // ファイル名
    pub url: Option<String>,       // URL
    pub note: Option<String>,      // 備考
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
}
```

### 拡張後の DocStatus
```rust
pub struct DocStatus {
    pub status: bool,
    pub file: Option<String>,
    pub url: Option<String>,
    pub note: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    // ↓ 新規追加
    pub check_result: Option<CheckResultData>,  // AIチェック結果
    pub last_checked: Option<String>,           // 最終チェック日時 (ISO8601)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResultData {
    pub status: String,              // "ok" | "warning" | "error"
    pub summary: String,             // 1行サマリー
    pub items: Vec<CheckItem>,       // 詳細項目
    pub missing_fields: Vec<MissingField>, // 未記入項目
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    pub item_type: String,  // "ok" | "warning" | "error"
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingField {
    pub field: String,
    pub location: String,
}
```

## API設計

### エンドポイント一覧

| Method | Path | 説明 | Request | Response |
|--------|------|------|---------|----------|
| GET | `/health` | ヘルスチェック | - | `{"status": "ok"}` |
| GET | `/doc-types` | 書類タイプ一覧 | - | `{"doc_types": [...]}` |
| POST | `/check/url` | URLから書類チェック | `{url, doc_type, contractor}` | CheckResult |
| POST | `/ocr/url` | URLからOCR | `{url}` | OcrResult |

### リクエスト/レスポンス例

```json
// POST /check/url
// Request
{
  "url": "https://drive.google.com/file/d/xxx/view",
  "doc_type": "暴対法誓約書",
  "contractor": "〇〇建設"
}

// Response
{
  "status": "warning",
  "summary": "署名欄が未記入です",
  "items": [
    {"type": "ok", "message": "日付が記入されています"},
    {"type": "warning", "message": "署名欄が空欄です"}
  ],
  "missing_fields": [
    {"field": "署名", "location": "下部"}
  ]
}
```

## UIフロー

### 1. ダッシュボード
- 書類カードに所見バッジ表示（✓ OK / ⚠ 警告 / ✗ エラー）
- クリックでDocViewerへ遷移

### 2. DocViewer (PDF/Spreadsheet共通)
- 上部: 戻るボタン、書類名、外部リンク
- 中央: プレビュー (iframe)
- 下部: アクションバー
  - [OCR実行] ボタン
  - [AIチェック] ボタン
  - [保存] ボタン

### 3. ResultPanel (DocViewer内に表示)
- チェック結果サマリー
- 詳細項目リスト
- 未記入項目リスト
- 備考編集フィールド

## 分割タスク一覧

| ID | タスク名 | 言語 | 依存 | 優先度 |
|----|----------|------|------|--------|
| T1 | データ構造拡張 | Rust | - | 高 |
| T2 | API通信モジュール | Rust | T1 | 高 |
| T3 | サーバーURL対応 | Python | - | 高 |
| T4 | ビューワアクションUI | Rust | T1,T2 | 中 |
| T5 | 結果パネルUI | Rust | T1 | 中 |
| T6 | ダッシュボード所見表示 | Rust | T1 | 中 |
| T7 | 保存機能拡張 | Rust | T1 | 低 |

## 実行計画

```
Round 1 (並列):
├── T1: データ構造拡張 (Rust)
└── T3: サーバーURL対応 (Python)

Round 2 (並列):
├── T2: API通信モジュール (Rust) ※T1完了後
├── T5: 結果パネルUI (Rust)
└── T6: ダッシュボード所見表示 (Rust)

Round 3:
├── T4: ビューワアクションUI (Rust) ※T1,T2完了後

Round 4:
└── T7: 保存機能拡張 (Rust)
└── 統合テスト
```
