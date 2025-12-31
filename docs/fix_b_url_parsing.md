# Fix B: Google Drive/Sheets URLパースの堅牢化

## 問題
1. Google Drive URLパースが `replace("/view", "/preview")` など文字列置換に依存しており脆弱
2. Google Sheets埋め込みURL構築が複数の `replace` 呼び出しで予期せぬ動作の原因に

## 修正箇所
`src/main.rs` の以下の関数/箇所:
1. `PdfViewer` コンポーネント内のGoogle Drive URL変換
2. `SpreadsheetViewer` コンポーネント内のGoogle Sheets埋め込みURL構築

## 修正1: Google DriveファイルID抽出関数の追加

```rust
/// Google Drive URLからファイルIDを抽出
fn extract_drive_file_id(url: &str) -> Option<String> {
    // パターン: /d/{file_id}/ または /d/{file_id}
    if let Some(start) = url.find("/d/") {
        let after_d = &url[start + 3..];
        let end = after_d.find('/').unwrap_or(after_d.len());
        let file_id = &after_d[..end];
        // クエリパラメータを除去
        let file_id = file_id.split('?').next().unwrap_or(file_id);
        if !file_id.is_empty() {
            return Some(file_id.to_string());
        }
    }
    None
}

/// Google DriveファイルIDからプレビューURLを構築
fn build_drive_preview_url(file_id: &str) -> String {
    format!("https://drive.google.com/file/d/{}/preview", file_id)
}
```

## 修正2: Google SheetsスプレッドシートID抽出と埋め込みURL構築

```rust
/// Google Sheets URLからスプレッドシートIDとgidを抽出
fn extract_spreadsheet_info(url: &str) -> Option<(String, Option<String>)> {
    // パターン: /spreadsheets/d/{spreadsheet_id}/
    if let Some(start) = url.find("/d/") {
        let after_d = &url[start + 3..];
        let end = after_d.find('/').unwrap_or(after_d.len());
        let spreadsheet_id = &after_d[..end];

        // gidを抽出（あれば）
        let gid = url.find("gid=").map(|pos| {
            let after_gid = &url[pos + 4..];
            let end = after_gid.find('&').unwrap_or(after_gid.len());
            after_gid[..end].to_string()
        });

        if !spreadsheet_id.is_empty() {
            return Some((spreadsheet_id.to_string(), gid));
        }
    }
    None
}

/// Google Sheets埋め込みURLを構築
fn build_sheets_embed_url(spreadsheet_id: &str, gid: Option<&str>) -> String {
    match gid {
        Some(g) => format!(
            "https://docs.google.com/spreadsheets/d/{}/htmlembed?gid={}",
            spreadsheet_id, g
        ),
        None => format!(
            "https://docs.google.com/spreadsheets/d/{}/htmlembed",
            spreadsheet_id
        ),
    }
}
```

## 修正3: PdfViewerでの使用

```rust
// 現在のコード（脆弱）
let preview_url = url.replace("/view", "/preview");

// 修正後
let preview_url = extract_drive_file_id(&url)
    .map(|id| build_drive_preview_url(&id))
    .unwrap_or_else(|| url.clone());
```

## 修正4: SpreadsheetViewerでの使用

```rust
// 現在のコード（脆弱）
let embed_url = if url.contains("docs.google.com/spreadsheets") {
    let base_url = url
        .replace("/edit", "")
        .replace("/view", "")
        // ...複雑な置換ロジック
};

// 修正後
let embed_url = if url.contains("docs.google.com/spreadsheets") {
    extract_spreadsheet_info(&url)
        .map(|(id, gid)| build_sheets_embed_url(&id, gid.as_deref()))
        .unwrap_or_else(|| url.clone())
} else {
    url.clone()
};
```

## テスト方法
1. 様々な形式のGoogle Drive URLでPdfViewerが正しく表示されることを確認
   - `https://drive.google.com/file/d/xxx/view`
   - `https://drive.google.com/file/d/xxx/view?usp=sharing`
   - `https://drive.google.com/open?id=xxx`

2. 様々な形式のGoogle Sheets URLでSpreadsheetViewerが正しく表示されることを確認
   - `https://docs.google.com/spreadsheets/d/xxx/edit`
   - `https://docs.google.com/spreadsheets/d/xxx/edit#gid=123`
   - `https://docs.google.com/spreadsheets/d/xxx/view?gid=456`
