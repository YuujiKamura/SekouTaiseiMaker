# Task T2: API通信モジュール (Rust)

## 概要
WASMからPythonバックエンド (gemini_server.py) へHTTP通信するモジュールを追加。

## 修正ファイル
- `src/main.rs`

## 前提条件
- T1 (データ構造拡張) が完了していること

## 修正内容

### 1. APIクライアント設定の定数を追加

```rust
// ============================================
// APIクライアント設定
// ============================================

/// ローカル開発用のAPIサーバーURL
const API_BASE_URL: &str = "http://localhost:5000";
```

### 2. API通信用の構造体を追加

```rust
/// チェックAPIリクエスト
#[derive(Debug, Clone, Serialize)]
pub struct CheckRequest {
    pub url: String,
    pub doc_type: String,
    pub contractor: String,
}

/// チェックAPIレスポンス（CheckResultDataと同じ形式）
#[derive(Debug, Clone, Deserialize)]
pub struct CheckResponse {
    pub status: String,
    pub summary: String,
    #[serde(default)]
    pub items: Vec<CheckItem>,
    #[serde(default)]
    pub missing_fields: Vec<MissingField>,
}

/// APIエラーレスポンス
#[derive(Debug, Clone, Deserialize)]
pub struct ApiError {
    pub error: String,
}
```

### 3. API通信関数を追加

```rust
/// サーバーのヘルスチェック
async fn check_api_health() -> Result<bool, String> {
    let url = format!("{}/health", API_BASE_URL);

    let opts = RequestInit::new();
    opts.set_method("GET");

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Request作成失敗: {:?}", e))?;

    let window = web_sys::window().ok_or("windowがありません")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch失敗: {:?}", e))?;

    let resp: Response = resp_value.dyn_into()
        .map_err(|_| "Responseへの変換失敗")?;

    Ok(resp.ok())
}

/// 書類チェックAPIを呼び出し
async fn call_check_api(req: CheckRequest) -> Result<CheckResultData, String> {
    let url = format!("{}/check/url", API_BASE_URL);

    let body = serde_json::to_string(&req)
        .map_err(|e| format!("JSON変換失敗: {:?}", e))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body));

    let headers = web_sys::Headers::new()
        .map_err(|_| "Headers作成失敗")?;
    headers.set("Content-Type", "application/json")
        .map_err(|_| "Header設定失敗")?;
    opts.set_headers(&headers);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Request作成失敗: {:?}", e))?;

    let window = web_sys::window().ok_or("windowがありません")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch失敗: {:?}", e))?;

    let resp: Response = resp_value.dyn_into()
        .map_err(|_| "Responseへの変換失敗")?;

    if !resp.ok() {
        let json = JsFuture::from(resp.json().map_err(|_| "json()失敗")?)
            .await
            .map_err(|_| "JSON解析失敗")?;
        let error: ApiError = serde_wasm_bindgen::from_value(json)
            .map_err(|_| "エラーレスポンス解析失敗")?;
        return Err(error.error);
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("json()失敗: {:?}", e))?)
        .await
        .map_err(|e| format!("JSON解析失敗: {:?}", e))?;

    let response: CheckResponse = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("デシリアライズ失敗: {:?}", e))?;

    // CheckResponseをCheckResultDataに変換
    Ok(CheckResultData {
        status: response.status,
        summary: response.summary,
        items: response.items,
        missing_fields: response.missing_fields,
    })
}
```

### 4. ProjectContextにAPI状態を追加

```rust
#[derive(Clone)]
pub struct ProjectContext {
    // ... 既存フィールド ...

    /// APIサーバー接続状態
    pub api_connected: ReadSignal<bool>,
    pub set_api_connected: WriteSignal<bool>,
    /// API処理中フラグ
    pub api_loading: ReadSignal<bool>,
    pub set_api_loading: WriteSignal<bool>,
}
```

### 5. App()でシグナルを初期化

```rust
// App()内
let (api_connected, set_api_connected) = create_signal(false);
let (api_loading, set_api_loading) = create_signal(false);

// 起動時にヘルスチェック
spawn_local(async move {
    match check_api_health().await {
        Ok(true) => set_api_connected.set(true),
        _ => set_api_connected.set(false),
    }
});
```

## テスト方法

```bash
# 1. Pythonサーバー起動
cd scripts
python gemini_server.py

# 2. WASMビルド＆起動
trunk serve

# 3. ブラウザで開き、コンソールでAPI接続状態を確認
```

## 依存関係
- T1 (データ構造拡張) 完了後に実行
