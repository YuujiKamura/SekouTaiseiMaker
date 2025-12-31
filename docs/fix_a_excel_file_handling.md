# Fix A: Excelファイル処理の修正

## 問題
ExcelファイルがSpreadsheetViewerに送られるが、SpreadsheetViewerはGoogle Sheetsのiframe埋め込みのみ対応。
Excelファイルを開こうとすると意図しないダウンロードが発生する。

## 修正箇所
`src/main.rs` の `detect_file_type` 使用箇所

## 現在のコード（問題あり）
```rust
DocFileType::GoogleSpreadsheet | DocFileType::Excel => {
    set_view_mode.set(ViewMode::SpreadsheetViewer {
        contractor: contractor_name.clone(),
        doc_type: doc_type_name.clone(),
        url: url.clone(),
    });
}
```

## 修正後のコード
```rust
DocFileType::GoogleSpreadsheet => {
    set_view_mode.set(ViewMode::SpreadsheetViewer {
        contractor: contractor_name.clone(),
        doc_type: doc_type_name.clone(),
        url: url.clone(),
    });
}
DocFileType::Excel => {
    // Excelは新規タブで開く（ローカルファイルのため埋め込み不可）
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(&url, "_blank");
    }
}
```

## 検索方法
1. `grep -n "DocFileType::Excel" src/main.rs` でExcel処理箇所を検索
2. SpreadsheetViewerと一緒に処理されている箇所を修正

## テスト方法
1. ダッシュボードでExcelファイルのセルをクリック
2. 新規タブでファイルが開かれることを確認（SpreadsheetViewerが開かないこと）
