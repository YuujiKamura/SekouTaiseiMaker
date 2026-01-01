# Rust PDFフォント埋め込み PoC

## 目的
RustでPDFに日本語フォント（OTF）を埋め込んでテキストを書けるか検証する

## 成功条件
1. 既存PDFを読み込む
2. 日本語テキスト「施工体制台帳」を追加
3. フォント埋め込み済みPDFとして保存
4. Adobe Readerで正しく表示される

## 検証するライブラリ

### 候補1: printpdf
```toml
[dependencies]
printpdf = "0.7"
```
- PDF生成に特化
- フォント埋め込みサポートあり
- 既存PDF編集は弱い？

### 候補2: lopdf + 手動フォント埋め込み
```toml
[dependencies]
lopdf = "0.31"
```
- 低レベルPDF操作
- フォント埋め込みは自分で実装

### 候補3: pdf-rs
```toml
[dependencies]
pdf = "0.8"
```
- 読み取り専用？要調査

## PoC実装計画

### ステップ1: printpdfで新規PDF作成（10分）
```rust
use printpdf::*;
use std::fs::File;

fn main() {
    let (doc, page1, layer1) = PdfDocument::new("Test", Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc.add_external_font(File::open("fonts/NotoSerifJP-Subset.otf").unwrap()).unwrap();
    let current_layer = doc.get_page(page1).get_layer(layer1);

    current_layer.use_text("施工体制台帳", 24.0, Mm(10.0), Mm(280.0), &font);

    doc.save(&mut BufWriter::new(File::create("output.pdf").unwrap())).unwrap();
}
```

### ステップ2: 結果確認
- [ ] PDFが生成される
- [ ] 日本語が表示される
- [ ] フォントが埋め込まれている（Adobe Readerで確認）

### ステップ3: 既存PDF編集（printpdfで無理なら別の方法）
- lopdfで既存PDF読み込み
- 新しいページコンテンツにテキスト追加
- フォントリソース追加

## 実行コマンド
```bash
cd rust-font-poc
cargo run
```

## 判定

| 結果 | 次のアクション |
|------|---------------|
| printpdfで成功 | 本格実装へ（既存PDF編集の方法を調査） |
| lopdfで成功 | 本格実装へ |
| 両方失敗 | JS(pdf-lib)のままで保守性改善を検討 |
