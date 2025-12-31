# Task T1: データ構造拡張 (Rust)

## 概要
DocStatusにAIチェック結果を保持するフィールドを追加する。

## 修正ファイル
- `src/main.rs`

## 修正内容

### 1. 新しい構造体を追加 (DocStatus定義の近くに)

```rust
/// AIチェック結果データ
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckResultData {
    /// "ok" | "warning" | "error"
    pub status: String,
    /// 1行サマリー
    pub summary: String,
    /// 詳細チェック項目
    #[serde(default)]
    pub items: Vec<CheckItem>,
    /// 未記入フィールド
    #[serde(default)]
    pub missing_fields: Vec<MissingField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    /// "ok" | "warning" | "error" | "info"
    #[serde(rename = "type")]
    pub item_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingField {
    pub field: String,
    pub location: String,
}
```

### 2. DocStatusにフィールド追加

```rust
// 現在
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocStatus {
    pub status: bool,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub valid_from: Option<String>,
    #[serde(default)]
    pub valid_until: Option<String>,
}

// 修正後
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocStatus {
    pub status: bool,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub valid_from: Option<String>,
    #[serde(default)]
    pub valid_until: Option<String>,
    /// AIチェック結果
    #[serde(default)]
    pub check_result: Option<CheckResultData>,
    /// 最終チェック日時 (ISO8601形式)
    #[serde(default)]
    pub last_checked: Option<String>,
}
```

### 3. DocEditorのmake_status関数を更新

DocEditor内の `make_status` クロージャで、新フィールドを保持するよう修正:

```rust
let make_status = move || DocStatus {
    status: doc_status.get(),
    file: if file.get().is_empty() { None } else { Some(file.get()) },
    url: if url.get().is_empty() { None } else { Some(url.get()) },
    note: if note.get().is_empty() { None } else { Some(note.get()) },
    valid_from: None,
    valid_until: if valid_until.get().is_empty() { None } else { Some(valid_until.get()) },
    // 既存の値を保持（編集時に消えないように）
    check_result: None,  // TODO: 既存値を保持する場合は引数から受け取る
    last_checked: None,
};
```

## テスト方法

```bash
# ビルドが通ることを確認
trunk build

# 既存のJSONデータが読み込めることを確認（後方互換性）
# 新フィールドがないJSONでもエラーにならないこと
```

## 注意事項
- `#[serde(default)]` により、既存JSONとの後方互換性を維持
- `#[serde(rename = "type")]` により、Pythonからの `"type"` キーを `item_type` にマッピング

## 依存関係
- なし（最初に実行可能）
