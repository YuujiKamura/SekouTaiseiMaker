# Task A: PDFビューワコンポーネント

## 目的
PDFファイルをプレビュー表示し、OCR実行や不足項目入力ができるビューワを作成する。

## 技術スタック
- Rust / Leptos 0.6 (CSR mode)
- WASM (trunk build)
- web-sys for DOM操作

## 作成するコンポーネント

### `PdfViewer` コンポーネント

```rust
#[derive(Clone)]
pub struct PdfViewerContext {
    pub pdf_url: RwSignal<String>,           // Google Drive URL
    pub pdf_blob_url: RwSignal<Option<String>>, // ローカルblob URL
    pub ocr_result: RwSignal<Option<OcrResult>>,
    pub missing_fields: RwSignal<Vec<MissingField>>,
    pub gemini_check_result: RwSignal<Option<String>>,
    pub is_loading: RwSignal<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MissingField {
    pub field_name: String,
    pub field_type: String, // "date", "text", "signature"
    pub value: String,
    pub position: Option<FieldPosition>, // OCRで検出した位置
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FieldPosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

### UIレイアウト

```
+------------------------------------------+
|  [戻る]  書類名: 暴対法誓約書  [閉じる]    |
+------------------------------------------+
|                    |  操作パネル          |
|                    |  [OCR実行]          |
|   PDFプレビュー     |  [GEMINIチェック]    |
|   (iframe/canvas)  |                     |
|                    |  --- 不足項目 ---    |
|                    |  日付: [____]       |
|                    |  署名: [____]       |
|                    |                     |
|                    |  [PDF出力]          |
+------------------------------------------+
```

### 必要な機能

1. **PDFプレビュー**
   - Google DriveのPDF URLからファイルIDを抽出
   - `https://drive.google.com/file/d/{FILE_ID}/preview` でiframe表示
   - または、fetch + blob URLでobject要素表示

2. **OCR実行ボタン**
   - クリックでDocument AI OCRを呼び出す（Python経由）
   - 結果をOcrResultとして保存
   - 検出したフィールド位置を表示

3. **不足項目入力**
   - OCR結果から未入力項目を検出
   - 入力フォームを動的生成
   - 入力値を保存

4. **GEMINIチェック**
   - PDFの内容をGEMINIに送信
   - チェック結果を表示（不備・警告など）

## 既存コードとの統合

`src/main.rs` の `ViewMode` enumに追加:
```rust
pub enum ViewMode {
    Dashboard,
    OcrViewer,
    PdfViewer(String), // contractor_name_doc_type
}
```

## CSS追加 (style.css)

```css
.pdf-viewer {
    display: flex;
    height: calc(100vh - 60px);
}

.pdf-preview-area {
    flex: 1;
    background: #333;
    display: flex;
    align-items: center;
    justify-content: center;
}

.pdf-preview-area iframe {
    width: 100%;
    height: 100%;
    border: none;
}

.pdf-controls {
    width: 300px;
    padding: 20px;
    background: #f5f5f5;
    overflow-y: auto;
}

.control-button {
    width: 100%;
    padding: 12px;
    margin-bottom: 10px;
    background: #2196F3;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
}

.control-button:hover {
    background: #1976D2;
}

.missing-field {
    margin-bottom: 15px;
}

.missing-field label {
    display: block;
    margin-bottom: 5px;
    font-weight: bold;
}

.missing-field input {
    width: 100%;
    padding: 8px;
    border: 1px solid #ddd;
    border-radius: 4px;
}
```

## 入力ファイル
- `src/main.rs` - 既存のLeptosアプリ
- `style.css` - 既存スタイル

## 出力
- `PdfViewer`コンポーネントを`src/main.rs`に追加
- 対応するCSSを`style.css`に追加

## 注意事項
- Google Drive PDFはCORS制限があるため、iframeのpreview URLを使用
- OCRとGEMINIは別タスク（Task D）で実装するAPI関数を呼び出す形にする
- 今回はUI部分のみ実装し、API呼び出しはダミー関数でOK
