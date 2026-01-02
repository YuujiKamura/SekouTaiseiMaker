//! GAS (Google Apps Script) 連携

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};
use serde::Deserialize;
use crate::models::ProjectData;

const GAS_URL_KEY: &str = "sekou_taisei_gas_url";

/// GASスクリプトの更新日時を取得（ビルド時に埋め込み）
pub fn format_gas_modified_time() -> String {
    let timestamp_str = option_env!("GAS_SCRIPT_MODIFIED").unwrap_or("0");
    let timestamp: i64 = timestamp_str.parse().unwrap_or(0);
    if timestamp == 0 {
        return "GASコード更新: 不明".to_string();
    }
    // JST (UTC+9) に変換して表示
    let secs = timestamp + 9 * 3600;
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    // 1970-01-01 からの日数を年月日に変換（簡易計算）
    let (year, month, day) = days_to_ymd(days);
    format!("GASコード更新: {}-{:02}-{:02} {:02}:{:02}", year, month, day, hours, minutes)
}

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    // 簡易的なグレゴリオ暦変換
    let mut remaining = days;
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for &d in &days_in_months {
        if remaining < d {
            break;
        }
        remaining -= d;
        month += 1;
    }
    (year, month, remaining + 1)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// GAS URLを保存
pub fn save_gas_url(url: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(GAS_URL_KEY, url);
        }
    }
}

/// GAS URLを取得
pub fn get_gas_url() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let url = storage.get_item(GAS_URL_KEY).ok()??;
    if url.is_empty() { None } else { Some(url) }
}

/// GAS URLをクリア
pub fn clear_gas_url() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(GAS_URL_KEY);
        }
    }
}

/// URLパラメータからGAS URLを読み込む (?gas=xxx)
pub fn init_gas_from_url_params() -> Option<String> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    if search.starts_with("?gas=") {
        let encoded = &search[5..];
        let decoded = js_sys::decode_uri_component(encoded).ok()?.as_string()?;
        save_gas_url(&decoded);
        // URLからパラメータを削除
        let pathname = window.location().pathname().ok()?;
        let hash = window.location().hash().ok().unwrap_or_default();
        let _ = window.history().unwrap().replace_state_with_url(
            &JsValue::NULL,
            "",
            Some(&format!("{}{}", pathname, hash))
        );
        Some(decoded)
    } else {
        None
    }
}

/// 共有URL生成（GAS URL付き）
pub fn generate_gas_share_url() -> Option<String> {
    let gas_url = get_gas_url()?;
    let window = web_sys::window()?;
    let location = window.location();
    let base_url = format!(
        "{}//{}{}",
        location.protocol().ok()?,
        location.host().ok()?,
        location.pathname().ok()?
    );
    let encoded = js_sys::encode_uri_component(&gas_url).as_string()?;
    Some(format!("{}?gas={}", base_url, encoded))
}

// GASレスポンス型
#[derive(Deserialize)]
struct GasResponse {
    project: Option<ProjectData>,
    #[allow(dead_code)]
    timestamp: Option<String>,
    #[allow(dead_code)]
    error: Option<String>,
    settings: Option<GasSettings>,
}

#[derive(Deserialize)]
struct GasSettings {
    #[serde(rename = "encryptedApiKey")]
    encrypted_api_key: Option<String>,
}

/// GASからプロジェクトデータを取得
pub async fn fetch_from_gas() -> Result<ProjectData, String> {
    let gas_url = get_gas_url().ok_or("GAS URLが設定されていません")?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let request = Request::new_with_str_and_init(&gas_url, &opts)
        .map_err(|e| format!("Request作成失敗: {:?}", e))?;

    let window = web_sys::window().ok_or("windowがありません")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch失敗: {:?}", e))?;

    let resp: Response = resp_value.dyn_into()
        .map_err(|_| "Responseへの変換失敗")?;

    if !resp.ok() {
        return Err(format!("APIエラー: {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("json()失敗: {:?}", e))?)
        .await
        .map_err(|e| format!("JSON取得失敗: {:?}", e))?;

    let response: GasResponse = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("JSONパース失敗: {:?}", e))?;

    // 暗号化APIキーがあれば復号してセット
    if let Some(ref settings) = response.settings {
        if let Some(ref encrypted) = settings.encrypted_api_key {
            if !encrypted.is_empty() {
                load_encrypted_api_key(encrypted).await;
            }
        }
    }

    response.project.ok_or("プロジェクトデータが空です".to_string())
}

/// GASにプロジェクトデータを保存
pub async fn save_to_gas(project: &ProjectData) -> Result<String, String> {
    let gas_url = get_gas_url().ok_or("GAS URLが設定されていません")?;

    let body = serde_json::json!({
        "action": "save",
        "project": project
    });

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body.to_string()));

    let request = Request::new_with_str_and_init(&gas_url, &opts)
        .map_err(|e| format!("Request作成失敗: {:?}", e))?;

    // Content-Type: text/plain を使ってCORSプリフライトを回避
    // GAS側はpostData.contentsをJSONとしてパースするので問題ない
    request.headers()
        .set("Content-Type", "text/plain")
        .map_err(|e| format!("ヘッダー設定失敗: {:?}", e))?;

    let window = web_sys::window().ok_or("windowがありません")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch失敗: {:?}", e))?;

    let resp: Response = resp_value.dyn_into()
        .map_err(|_| "Responseへの変換失敗")?;

    if !resp.ok() {
        return Err(format!("保存エラー: {}", resp.status()));
    }

    #[derive(Deserialize)]
    struct SaveResponse {
        #[allow(dead_code)]
        success: Option<bool>,
        timestamp: Option<String>,
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("json()失敗: {:?}", e))?)
        .await
        .map_err(|e| format!("JSON取得失敗: {:?}", e))?;

    let response: SaveResponse = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("JSONパース失敗: {:?}", e))?;

    Ok(response.timestamp.unwrap_or_else(|| "保存完了".to_string()))
}

/// 暗号化APIキーを読み込み（JS側の関数を呼び出し）
async fn load_encrypted_api_key(encrypted_data: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    if let Ok(func) = js_sys::Reflect::get(&window, &JsValue::from_str("loadEncryptedApiKey")) {
        if let Ok(func) = func.dyn_into::<js_sys::Function>() {
            if let Ok(promise) = func.call1(&JsValue::NULL, &JsValue::from_str(encrypted_data)) {
                if let Ok(promise) = promise.dyn_into::<js_sys::Promise>() {
                    let _ = JsFuture::from(promise).await;
                }
            }
        }
    }
}

/// APIキーをスプレッドシートに自動保存
pub async fn auto_save_api_key_to_sheet(gas_url: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    // APIキーがあるかチェック
    let has_key = js_sys::Reflect::get(&window, &JsValue::from_str("hasApiKey"))
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .and_then(|f| f.call0(&JsValue::NULL).ok())
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !has_key {
        return;
    }

    // saveApiKeyToSpreadsheet を呼び出し
    if let Ok(func) = js_sys::Reflect::get(&window, &JsValue::from_str("saveApiKeyToSpreadsheet")) {
        if let Ok(func) = func.dyn_into::<js_sys::Function>() {
            if let Ok(promise) = func.call1(&JsValue::NULL, &JsValue::from_str(gas_url)) {
                if let Ok(promise) = promise.dyn_into::<js_sys::Promise>() {
                    let _ = JsFuture::from(promise).await;
                }
            }
        }
    }
}
