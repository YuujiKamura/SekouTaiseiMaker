# Fix G: Google Sheets URL解析の堅牢化

## 問題
1. スプレッドシートID抽出時に `?usp=sharing` などのクエリパラメータが含まれてしまう
2. `gid=` の単純検索は他のパラメータ値と誤マッチする可能性がある
3. `#gid=` の処理ロジックが到達不能になる場合がある

## 修正ファイル
- `src/main.rs` (1024-1039行目付近の `SpreadsheetViewer` コンポーネント)

## 現在のコード

```rust
// Google Sheets URLを埋め込み用に変換
let embed_url = if url.contains("docs.google.com/spreadsheets") {
    // /edit や /view を /htmlembed に変換
    let base_url = url
        .replace("/edit", "")
        .replace("/view", "")
        .replace("#gid=", "/htmlembed?gid=");
    if base_url.contains("/htmlembed") {
        base_url
    } else if base_url.contains("?") {
        format!("{}&embedded=true", base_url)
    } else {
        format!("{}?embedded=true", base_url)
    }
} else {
    url.clone()
};
```

## 修正後のコード

```rust
/// Google SheetsのURLからスプレッドシートIDを抽出
fn extract_spreadsheet_id(url: &str) -> Option<String> {
    // パターン: /spreadsheets/d/{SPREADSHEET_ID}/...
    if let Some(start) = url.find("/d/") {
        let id_start = start + 3;
        let rest = &url[id_start..];
        // ID終端: '/', '?', '#' のいずれか
        let id_end = rest.find(|c| c == '/' || c == '?' || c == '#')
            .unwrap_or(rest.len());
        let id = &rest[..id_end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

/// URLからgidパラメータを抽出
fn extract_gid(url: &str) -> Option<String> {
    // #gid= または ?gid= または &gid= を探す
    for prefix in ["#gid=", "?gid=", "&gid="] {
        if let Some(start) = url.find(prefix) {
            let gid_start = start + prefix.len();
            let rest = &url[gid_start..];
            // gid終端: '&', '#', 空白のいずれか
            let gid_end = rest.find(|c: char| c == '&' || c == '#' || c.is_whitespace())
                .unwrap_or(rest.len());
            let gid = &rest[..gid_end];
            if !gid.is_empty() && gid.chars().all(|c| c.is_ascii_digit()) {
                return Some(gid.to_string());
            }
        }
    }
    None
}

/// Google Sheets URLを埋め込み用URLに変換
fn build_sheets_embed_url(url: &str) -> String {
    if !url.contains("docs.google.com/spreadsheets") {
        return url.to_string();
    }

    // スプレッドシートIDを抽出
    let spreadsheet_id = match extract_spreadsheet_id(url) {
        Some(id) => id,
        None => return url.to_string(), // ID抽出失敗時は元URLを返す
    };

    // gidを抽出（なければデフォルト0）
    let gid = extract_gid(url).unwrap_or_else(|| "0".to_string());

    // 埋め込みURL構築
    format!(
        "https://docs.google.com/spreadsheets/d/{}/htmlembed?gid={}",
        spreadsheet_id, gid
    )
}

// SpreadsheetViewer内での使用
let embed_url = build_sheets_embed_url(&url);
```

## SpreadsheetViewerコンポーネントの修正

```rust
#[component]
fn SpreadsheetViewer(
    contractor: String,
    doc_type: String,
    url: String,
) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");

    let on_back = move |_| {
        ctx.set_view_mode.set(ViewMode::Dashboard);
    };

    // 修正: 専用関数を使用
    let embed_url = build_sheets_embed_url(&url);

    view! {
        // ... 以下同じ
    }
}
```

## テスト方法
```rust
// テストケース（コメントとして確認用）
// 入力: https://docs.google.com/spreadsheets/d/ABC123/edit?usp=sharing
// 期待: https://docs.google.com/spreadsheets/d/ABC123/htmlembed?gid=0

// 入力: https://docs.google.com/spreadsheets/d/ABC123/edit#gid=456
// 期待: https://docs.google.com/spreadsheets/d/ABC123/htmlembed?gid=456

// 入力: https://docs.google.com/spreadsheets/d/ABC123/view?gid=789&other=param
// 期待: https://docs.google.com/spreadsheets/d/ABC123/htmlembed?gid=789
```

ビルド確認:
```bash
trunk build
```

## 依存関係
- 単独で実行可能
- 他のFixとは独立
