# PDF Editor Rust移行タスク

## 概要
pdf-editor.js (約830行) をRust + WASMに移行する

## 現在のJS実装の機能
- PDFの読み込み（File/Base64）
- ページ描画（Canvas）
- テキスト注釈（追加/選択/移動/削除/編集）
- フォント管理（明朝/ゴシック、サイズ、色）
- 永続化（LocalStorage）
- PDF保存（pdf-libでテキスト埋め込み）
- Google Driveアップロード

---

## フェーズ1: 基盤（並列実行可能）

### タスク1-1: プロジェクト構造設計
**依存**: なし
**説明**: Rustプロジェクトの構造を設計する

```
src/
  lib.rs          - エントリポイント、WASM公開API
  pdf/
    mod.rs
    loader.rs     - PDF読み込み
    saver.rs      - PDF保存
  editor/
    mod.rs
    state.rs      - エディタ状態管理
    annotation.rs - 注釈データ構造
    selection.rs  - 選択・ドラッグ
  render/
    mod.rs
    canvas.rs     - Canvas描画
  font/
    mod.rs
    loader.rs     - フォント読み込み
    embed.rs      - フォント埋め込み
  storage/
    mod.rs        - LocalStorage/IndexedDB
```

**Cargo.toml依存関係**:
```toml
[dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [...] }
js-sys = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
lopdf = "0.31"  # または pdf-rs
```

**出力**: プロジェクト構造とCargo.toml

---

### タスク1-2: データ構造定義
**依存**: なし
**説明**: 注釈とエディタ状態のRust構造体を定義

```rust
// annotation.rs
#[derive(Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    pub id: String,
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub font_size: f64,
    pub font_family: FontFamily,
    pub color: String,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum FontFamily {
    Mincho,
    Gothic,
}

// state.rs
pub struct EditorState {
    pub current_page: u32,
    pub total_pages: u32,
    pub scale: f64,
    pub annotations: Vec<TextAnnotation>,
    pub selected_id: Option<String>,
    pub hovered_id: Option<String>,
    pub edit_mode: EditMode,
    pub current_font_size: f64,
    pub current_font_family: FontFamily,
    pub current_color: String,
    // ドラッグ状態
    pub is_dragging: bool,
    pub drag_offset: (f64, f64),
}

pub enum EditMode {
    Add,
    Select,
}
```

**出力**: annotation.rs, state.rs

---

### タスク1-3: PDF読み込み
**依存**: なし
**説明**: lopdfまたはpdf-rsでPDFバイト列を読み込む

```rust
// loader.rs
use lopdf::Document;

pub struct PdfDocument {
    doc: Document,
    bytes: Vec<u8>,
    pdf_id: String,
}

impl PdfDocument {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, PdfError> {
        let doc = Document::load_mem(&bytes)?;
        let pdf_id = generate_pdf_id(&bytes);
        Ok(Self { doc, bytes, pdf_id })
    }

    pub fn page_count(&self) -> u32 {
        self.doc.get_pages().len() as u32
    }

    pub fn get_page_size(&self, page: u32) -> (f64, f64) {
        // ページサイズ取得
    }

    pub fn get_page_rotation(&self, page: u32) -> u32 {
        // ページ回転取得
    }
}

fn generate_pdf_id(bytes: &[u8]) -> String {
    // サイズ + 先頭バイトのハッシュ
}
```

**注意**: lopdfはPDF解析のみ。描画はpdf.js（JS側）に任せる

**出力**: loader.rs

---

## フェーズ2: コア機能（フェーズ1完了後）

### タスク2-1: 注釈管理
**依存**: タスク1-2
**説明**: 注釈の追加、削除、更新、検索

```rust
// editor/mod.rs
impl EditorState {
    pub fn add_annotation(&mut self, x: f64, y: f64, text: String) -> TextAnnotation {
        let id = format!("ann_{}", self.annotation_counter);
        self.annotation_counter += 1;
        let dims = self.measure_text(&text, self.current_font_size);
        let ann = TextAnnotation {
            id: id.clone(),
            page: self.current_page,
            x: x / self.scale,
            y: y / self.scale,
            text,
            font_size: self.current_font_size,
            font_family: self.current_font_family,
            color: self.current_color.clone(),
            width: dims.0,
            height: dims.1,
        };
        self.annotations.push(ann.clone());
        ann
    }

    pub fn delete_selected(&mut self) -> bool { ... }
    pub fn update_selected_text(&mut self, text: String) -> bool { ... }
    pub fn undo_last(&mut self) -> bool { ... }
}
```

**出力**: editor/mod.rsの注釈管理部分

---

### タスク2-2: 選択・ドラッグ・ヒット判定
**依存**: タスク1-2, 2-1
**説明**: 座標からの注釈検索、選択、ドラッグ移動

```rust
// selection.rs
impl EditorState {
    pub fn get_annotation_at(&self, screen_x: f64, screen_y: f64) -> Option<&TextAnnotation> {
        let x = screen_x / self.scale;
        let y = screen_y / self.scale;
        let margin = 5.0;

        self.annotations
            .iter()
            .filter(|a| a.page == self.current_page)
            .rev()  // 後から追加されたものを優先
            .find(|a| {
                x >= a.x - margin && x <= a.x + a.width + margin &&
                y >= a.y - a.height - margin && y <= a.y + margin
            })
    }

    pub fn select(&mut self, id: Option<String>) { ... }
    pub fn start_drag(&mut self, x: f64, y: f64) -> bool { ... }
    pub fn update_drag(&mut self, x: f64, y: f64) { ... }
    pub fn end_drag(&mut self) { ... }
    pub fn nudge_selected(&mut self, dx: f64, dy: f64) -> bool { ... }
}
```

**出力**: selection.rs

---

### タスク2-3: 永続化（LocalStorage）
**依存**: タスク1-2
**説明**: 注釈のLocalStorage保存/読み込み

```rust
// storage/mod.rs
use wasm_bindgen::prelude::*;
use web_sys::Storage;

const STORAGE_PREFIX: &str = "pdfEditor_annotations_";

pub fn save_annotations(pdf_id: &str, annotations: &[TextAnnotation], counter: u32) -> Result<(), JsValue> {
    let storage = get_local_storage()?;
    let data = StorageData { annotations, counter, saved_at: now() };
    let json = serde_json::to_string(&data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    storage.set_item(&format!("{}{}", STORAGE_PREFIX, pdf_id), &json)?;
    Ok(())
}

pub fn load_annotations(pdf_id: &str) -> Option<(Vec<TextAnnotation>, u32)> {
    let storage = get_local_storage().ok()?;
    let json = storage.get_item(&format!("{}{}", STORAGE_PREFIX, pdf_id)).ok()??;
    let data: StorageData = serde_json::from_str(&json).ok()?;
    Some((data.annotations, data.counter))
}
```

**出力**: storage/mod.rs

---

## フェーズ3: 描画（フェーズ2完了後）

### タスク3-1: Canvas描画
**依存**: タスク2-1, 2-2
**説明**: オーバーレイCanvasへの注釈描画

```rust
// render/canvas.rs
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub struct CanvasRenderer {
    ctx: CanvasRenderingContext2d,
    scale: f64,
    dpr: f64,
}

impl CanvasRenderer {
    pub fn clear(&self) { ... }

    pub fn draw_annotations(&self, state: &EditorState) {
        self.clear();
        for ann in state.annotations.iter().filter(|a| a.page == state.current_page) {
            self.draw_annotation(ann, state.selected_id.as_ref(), state.hovered_id.as_ref());
        }
    }

    fn draw_annotation(&self, ann: &TextAnnotation, selected: Option<&String>, hovered: Option<&String>) {
        let screen_x = ann.x * self.scale;
        let screen_y = ann.y * self.scale;

        // 選択/ホバー時の背景・枠
        if Some(&ann.id) == selected {
            self.draw_selection_box(screen_x, screen_y, ann.width * self.scale, ann.height * self.scale);
        } else if Some(&ann.id) == hovered {
            self.draw_hover_box(...);
        }

        // テキスト描画
        self.draw_text(ann.text, screen_x, screen_y, ann.font_size, &ann.font_family, &ann.color);
    }
}
```

**注意**: pdf.js描画は引き続きJS側で行う。Rustはオーバーレイのみ担当

**出力**: render/canvas.rs

---

### タスク3-2: フォント読み込み
**依存**: なし（並列可能）
**説明**: OTFフォントファイルの読み込みとキャッシュ

```rust
// font/loader.rs
use std::collections::HashMap;

pub struct FontCache {
    fonts: HashMap<FontFamily, Vec<u8>>,
}

impl FontCache {
    pub async fn load_font(&mut self, family: FontFamily) -> Result<&[u8], JsValue> {
        if self.fonts.contains_key(&family) {
            return Ok(self.fonts.get(&family).unwrap());
        }

        let url = match family {
            FontFamily::Gothic => "./fonts/NotoSansJP-Subset.otf",
            FontFamily::Mincho => "./fonts/NotoSerifJP-Subset.otf",
        };

        let bytes = fetch_bytes(url).await?;
        self.fonts.insert(family, bytes);
        Ok(self.fonts.get(&family).unwrap())
    }
}
```

**出力**: font/loader.rs

---

## フェーズ4: PDF保存（フェーズ2, 3完了後）

### タスク4-1: PDF保存（テキスト埋め込み）
**依存**: タスク1-3, 2-1, 3-2
**説明**: lopdfでPDFにテキストを埋め込んで保存

```rust
// pdf/saver.rs
impl PdfDocument {
    pub fn save_with_annotations(
        &self,
        annotations: &[TextAnnotation],
        fonts: &FontCache
    ) -> Result<Vec<u8>, PdfError> {
        let mut doc = self.doc.clone();

        for ann in annotations {
            let page = doc.get_page(ann.page)?;
            let (width, height) = self.get_page_size(ann.page);
            let rotation = self.get_page_rotation(ann.page);

            // 座標変換（回転考慮）
            let (pdf_x, pdf_y) = transform_coords(ann.x, ann.y, width, height, rotation);

            // フォント埋め込み
            let font_bytes = fonts.get(&ann.font_family)?;
            let font_id = self.embed_font(&mut doc, font_bytes)?;

            // テキスト描画コマンド追加
            self.add_text_to_page(&mut doc, ann.page, pdf_x, pdf_y, &ann.text, ann.font_size, font_id)?;
        }

        doc.save_to_bytes()
    }
}
```

**注意**: lopdfでのフォント埋め込みは複雑。pdf-rsやprintpdfの方が楽かも

**出力**: pdf/saver.rs

---

## フェーズ5: 統合（全フェーズ完了後）

### タスク5-1: WASM公開API
**依存**: 全タスク
**説明**: JavaScriptから呼び出すAPIを定義

```rust
// lib.rs
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct PdfEditor {
    state: EditorState,
    document: Option<PdfDocument>,
    font_cache: FontCache,
    renderer: Option<CanvasRenderer>,
}

#[wasm_bindgen]
impl PdfEditor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { ... }

    pub async fn load_pdf(&mut self, bytes: &[u8]) -> Result<JsValue, JsValue> { ... }
    pub fn render_page(&mut self, page: u32, canvas_id: &str) -> Result<(), JsValue> { ... }
    pub fn add_text(&mut self, x: f64, y: f64, text: &str) -> JsValue { ... }
    pub fn select_at(&mut self, x: f64, y: f64) -> Option<String> { ... }
    pub fn delete_selected(&mut self) -> bool { ... }
    pub fn start_drag(&mut self, x: f64, y: f64) -> bool { ... }
    pub fn update_drag(&mut self, x: f64, y: f64) { ... }
    pub fn end_drag(&mut self) { ... }
    pub async fn save_pdf(&self) -> Result<Vec<u8>, JsValue> { ... }

    // 設定
    pub fn set_font_size(&mut self, size: f64) { ... }
    pub fn set_font_family(&mut self, family: &str) { ... }
    pub fn set_color(&mut self, color: &str) { ... }
    pub fn set_edit_mode(&mut self, mode: &str) { ... }
}
```

**出力**: lib.rs

---

### タスク5-2: 既存HTMLとの統合
**依存**: タスク5-1
**説明**: index.htmlでRust WASMモジュールを使用するように変更

```javascript
// 初期化
import init, { PdfEditor } from './pkg/pdf_editor.js';

await init();
const editor = new PdfEditor();

// PDF読み込み
const bytes = new Uint8Array(await file.arrayBuffer());
await editor.load_pdf(bytes);

// ページ描画（pdf.jsはそのまま使う、オーバーレイのみRust）
await renderPageWithPdfJs(page);
editor.render_overlay(page, 'overlayCanvas');

// クリック処理
canvas.onclick = (e) => {
    if (editor.get_edit_mode() === 'add') {
        editor.add_text(e.offsetX, e.offsetY, inputText);
    } else {
        editor.select_at(e.offsetX, e.offsetY);
    }
};
```

**出力**: index.htmlの修正

---

## 実行順序まとめ

```
フェーズ1（並列可能）
├── タスク1-1: プロジェクト構造設計
├── タスク1-2: データ構造定義
└── タスク1-3: PDF読み込み

    ↓

フェーズ2（フェーズ1完了後、一部並列可能）
├── タスク2-1: 注釈管理 ← 1-2依存
├── タスク2-2: 選択・ドラッグ ← 1-2, 2-1依存
└── タスク2-3: 永続化 ← 1-2依存（2-1と並列可能）

    ↓

フェーズ3（フェーズ2完了後、一部並列可能）
├── タスク3-1: Canvas描画 ← 2-1, 2-2依存
└── タスク3-2: フォント読み込み ← 依存なし（先行可能）

    ↓

フェーズ4（フェーズ2, 3完了後）
└── タスク4-1: PDF保存 ← 1-3, 2-1, 3-2依存

    ↓

フェーズ5（全フェーズ完了後）
├── タスク5-1: WASM公開API ← 全タスク依存
└── タスク5-2: HTML統合 ← 5-1依存
```

## 注意事項

1. **pdf.jsは残す**: PDF描画（ラスタライズ）はpdf.jsの方が優秀。RustはPDFのパース・保存のみ担当
2. **lopdfの制限**: フォント埋め込みが弱い。必要に応じてprintpdfやpdf-rsを検討
3. **段階的移行**: 各フェーズ完了時にJSとRustを共存させてテスト可能
4. **フォント埋め込みが最難関**: 日本語フォントの埋め込みはRustライブラリの対応状況次第

## 各タスクの推定難易度

| タスク | 難易度 | 理由 |
|--------|--------|------|
| 1-1 | 低 | 構造設計のみ |
| 1-2 | 低 | データ構造定義のみ |
| 1-3 | 中 | lopdf APIの理解必要 |
| 2-1 | 低 | JSからの移植 |
| 2-2 | 低 | JSからの移植 |
| 2-3 | 中 | web-sys APIの理解必要 |
| 3-1 | 中 | web-sys Canvas APIの理解必要 |
| 3-2 | 低 | fetch + キャッシュ |
| 4-1 | **高** | フォント埋め込みが複雑 |
| 5-1 | 中 | wasm-bindgenの理解必要 |
| 5-2 | 低 | JS側の修正のみ |
