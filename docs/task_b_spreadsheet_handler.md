# Task B: スプレッドシート/Excel対応

## 目的
Google Spreadsheet や Excel ファイルを開く/確認する機能を作成する。

## 技術スタック
- Rust / Leptos 0.6 (CSR mode)
- web-sys for window.open
- Python (GEMINI API呼び出し用)

## 作成するコンポーネント

### `SpreadsheetViewer` コンポーネント

```rust
#[derive(Clone)]
pub struct SpreadsheetViewerContext {
    pub doc_url: RwSignal<String>,
    pub doc_type: RwSignal<SpreadsheetType>,
    pub gemini_check_result: RwSignal<Option<String>>,
    pub is_checking: RwSignal<bool>,
}

#[derive(Clone, PartialEq)]
pub enum SpreadsheetType {
    GoogleSpreadsheet,
    Excel,
    Unknown,
}
```

### UIレイアウト

```
+------------------------------------------+
|  [戻る]  書類名: 作業員名簿  [閉じる]      |
+------------------------------------------+
|                                          |
|   このファイルはスプレッドシートです        |
|                                          |
|   [ブラウザで開く]                        |
|                                          |
|   --- GEMINI確認 ---                     |
|   [内容をチェック]                        |
|                                          |
|   確認結果:                              |
|   +------------------------------------+ |
|   | ✓ 作業員名簿の必須項目が           | |
|   |   すべて入力されています            | |
|   | ⚠ 資格欄に記載漏れの              | |
|   |   可能性があります                 | |
|   +------------------------------------+ |
|                                          |
+------------------------------------------+
```

### 必要な機能

1. **URL判定**
   ```rust
   fn detect_spreadsheet_type(url: &str) -> SpreadsheetType {
       if url.contains("docs.google.com/spreadsheets") {
           SpreadsheetType::GoogleSpreadsheet
       } else if url.ends_with(".xlsx") || url.ends_with(".xls") {
           SpreadsheetType::Excel
       } else {
           SpreadsheetType::Unknown
       }
   }
   ```

2. **ブラウザで開く**
   ```rust
   fn open_in_browser(url: &str) {
       if let Some(window) = web_sys::window() {
           let _ = window.open_with_url_and_target(url, "_blank");
       }
   }
   ```

3. **GEMINI確認**
   - Sheets APIでスプレッドシート内容を取得
   - GEMINIに送信してチェック
   - 結果を表示

## Python側 API (別途実装)

```python
# check_spreadsheet.py
def check_spreadsheet_with_gemini(spreadsheet_id: str, check_type: str) -> dict:
    """
    スプレッドシートの内容をGEMINIでチェック

    check_type: "作業員名簿", "暴対法誓約書" など

    Returns:
        {
            "status": "ok" | "warning" | "error",
            "messages": ["メッセージ1", "メッセージ2"],
            "details": {...}
        }
    """
    pass
```

## 既存コードとの統合

`ViewMode` enumに追加:
```rust
pub enum ViewMode {
    Dashboard,
    OcrViewer,
    PdfViewer(String),
    SpreadsheetViewer(String), // contractor_name_doc_type
}
```

## CSS追加

```css
.spreadsheet-viewer {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 40px;
    max-width: 600px;
    margin: 0 auto;
}

.spreadsheet-icon {
    font-size: 64px;
    margin-bottom: 20px;
}

.spreadsheet-message {
    font-size: 18px;
    color: #666;
    margin-bottom: 30px;
}

.open-button {
    padding: 15px 40px;
    font-size: 16px;
    background: #34A853;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    margin-bottom: 30px;
}

.open-button:hover {
    background: #2E7D32;
}

.gemini-section {
    width: 100%;
    border-top: 1px solid #ddd;
    padding-top: 20px;
}

.check-result {
    margin-top: 15px;
    padding: 15px;
    background: #f9f9f9;
    border-radius: 4px;
    white-space: pre-wrap;
}

.check-result.ok {
    border-left: 4px solid #34A853;
}

.check-result.warning {
    border-left: 4px solid #FBBC04;
}

.check-result.error {
    border-left: 4px solid #EA4335;
}
```

## 出力
- `SpreadsheetViewer`コンポーネントを`src/main.rs`に追加
- 対応するCSSを`style.css`に追加

## 注意事項
- GEMINI API呼び出しは Task D で実装
- 今回はUI部分のみ実装し、API呼び出しはダミー関数でOK
