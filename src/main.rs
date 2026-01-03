// モジュール宣言
mod models;
mod utils;
mod components;
mod views;

// 外部クレート
use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{FileReader, HtmlInputElement, Request, RequestInit, Response};
use std::collections::HashMap;

// 自モジュールからのインポート
use models::*;
use components::{CheckResultTooltip, ContextMenu};
use utils::cache::{save_to_cache, load_from_cache, clear_cache};
use utils::gas::{get_gas_url, save_gas_url, clear_gas_url, init_gas_from_url_params, generate_gas_share_url, fetch_from_gas, auto_save_api_key_to_sheet, format_gas_modified_time, save_gas_url_to_sheet};
use utils::{encode_base64, decode_base64};
use utils::log_trace::{log_info, log_info_with_data, log_warn, log_error, log_error_with_data, download_logs, clear_logs, copy_logs_to_clipboard_async};
use views::{CheckResultsPanel, PdfViewer, SpreadsheetViewer};
use views::ocr_viewer::{OcrDocument, OcrToken, OcrViewContext, OcrViewer};
use components::{ProjectView, ProjectEditor};


// URLハッシュからデータを取得
fn get_hash_data() -> Option<ProjectData> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    if hash.starts_with("#data=") {
        let encoded = &hash[6..];
        let json = decode_base64(encoded)?;
        serde_json::from_str(&json).ok()
    } else {
        None
    }
}

/// GASにプロジェクトデータを保存
async fn sync_to_gas(project: &ProjectData) -> Result<String, String> {
    log_info("gas-sync", "GASへの保存を開始");
    let gas_url = get_gas_url().ok_or("GAS URLが設定されていません")?;

    #[derive(Serialize)]
    struct SaveRequest<'a> {
        action: &'static str,
        project: &'a ProjectData,
    }

    let body = serde_json::to_string(&SaveRequest {
        action: "save",
        project,
    }).map_err(|e| format!("JSON変換失敗: {:?}", e))?;

    // GASはCORSプリフライトに対応しないため、text/plainで送信
    // （Content-Type: application/jsonだとプリフライトが発生する）
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body));
    opts.set_mode(web_sys::RequestMode::Cors);

    let headers = web_sys::Headers::new().map_err(|_| "Headers作成失敗")?;
    headers.set("Content-Type", "text/plain").map_err(|_| "Header設定失敗")?;
    opts.set_headers(&headers);

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
        .map_err(|e| format!("JSON解析失敗: {:?}", e))?;

    #[derive(Deserialize)]
    struct SaveResponse {
        #[allow(dead_code)]
        success: Option<bool>,
        timestamp: Option<String>,
        error: Option<String>,
    }

    let response: SaveResponse = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("デシリアライズ失敗: {:?}", e))?;

    if let Some(err) = response.error {
        return Err(err);
    }

    Ok(response.timestamp.unwrap_or_else(|| "保存完了".to_string()))
}

// ============================================
// APIキー確認
// ============================================

const API_KEY_STORAGE_KEY: &str = "sekou_taisei_api_key";

// UI状態のエイリアス
pub use models::CheckResultTooltipState;
pub use models::ContextMenuState;

// チェック結果
#[derive(Debug, Clone, PartialEq)]
pub enum CheckMode {
    None,
    Existence,  // 書類存在チェック
    Date,       // 日付チェック
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub contractor_name: String,
    pub doc_name: String,
    pub status: CheckStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub contractor: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

// ============================================
// フィールドタイプとMissingField定義
// ============================================

/// 入力フィールドのタイプ
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    /// 日付入力
    Date,
    /// テキスト入力
    Text,
    /// 署名
    Signature,
    /// 選択肢
    Select,
    /// チェックボックス
    Checkbox,
}

impl FieldType {
    /// HTML input typeを取得
    pub fn input_type(&self) -> &'static str {
        match self {
            FieldType::Date => "date",
            FieldType::Text => "text",
            FieldType::Signature => "text", // 署名は別途処理
            FieldType::Select => "text",
            FieldType::Checkbox => "checkbox",
        }
    }

    /// プレースホルダーテキストを取得
    pub fn placeholder(&self) -> &'static str {
        match self {
            FieldType::Date => "YYYY-MM-DD",
            FieldType::Text => "入力してください",
            FieldType::Signature => "署名",
            FieldType::Select => "選択してください",
            FieldType::Checkbox => "",
        }
    }
}

/// フィールドの位置情報（OCRで検出した座標）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldPosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 不足フィールド情報
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissingField {
    pub field_name: String,
    pub field_type: FieldType,
    pub value: String,
    pub position: Option<FieldPosition>,
}

/// OCR結果（簡易版）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub pages: Vec<OcrPage>,
}

/// OCRページ情報
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OcrPage {
    pub page_number: u32,
    pub text: String,
}

impl Default for OcrResult {
    fn default() -> Self {
        OcrResult {
            text: String::new(),
            pages: Vec::new(),
        }
    }
}

// ============================================
// API通信関数
// ============================================

/// APIキー設定済みかチェック（localStorage）
fn check_api_key_exists() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return false,
    };
    match storage.get_item(API_KEY_STORAGE_KEY) {
        Ok(Some(key)) => key.starts_with("AIza") && key.len() >= 39,
        _ => false,
    }
}

// ============================================
// ダッシュボードコンポーネント
// ============================================

// グローバルなプロジェクトデータ用Context
#[derive(Clone)]
pub struct ProjectContext {
    pub project: ReadSignal<Option<ProjectData>>,
    pub set_project: WriteSignal<Option<ProjectData>>,
    pub loading: ReadSignal<bool>,
    pub set_loading: WriteSignal<bool>,
    pub error_msg: ReadSignal<Option<String>>,
    pub set_error_msg: WriteSignal<Option<String>>,
    pub check_mode: ReadSignal<CheckMode>,
    pub set_check_mode: WriteSignal<CheckMode>,
    pub check_results: ReadSignal<Vec<CheckResult>>,
    pub set_check_results: WriteSignal<Vec<CheckResult>>,
    pub edit_mode: ReadSignal<bool>,
    pub set_edit_mode: WriteSignal<bool>,
    pub view_mode: ReadSignal<ViewMode>,
    pub set_view_mode: WriteSignal<ViewMode>,
    /// APIサーバー接続状態
    pub api_connected: ReadSignal<bool>,
    pub set_api_connected: WriteSignal<bool>,
    /// API処理中フラグ
    pub api_loading: ReadSignal<bool>,
    pub set_api_loading: WriteSignal<bool>,
    /// チェック結果ツールチップ状態
    pub check_result_tooltip: ReadSignal<CheckResultTooltipState>,
    pub set_check_result_tooltip: WriteSignal<CheckResultTooltipState>,
    /// コンテキストメニュー状態（右クリック/ロングプレス）
    pub context_menu: ReadSignal<ContextMenuState>,
    pub set_context_menu: WriteSignal<ContextMenuState>,
}


#[component]
fn Dashboard() -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");

    view! {
        <div class="dashboard">
            {move || ctx.error_msg.get().map(|e| view! {
                <p class="status error">{e}</p>
            })}

            {move || {
                let edit_mode = ctx.edit_mode.get();
                ctx.project.get().map(|p| {
                    if edit_mode {
                        view! { <ProjectEditor project=p /> }.into_view()
                    } else {
                        view! { <ProjectView project=p /> }.into_view()
                    }
                })
            }}

            {move || ctx.project.get().is_none().then(|| view! {
                <div class="empty-state">
                    <p>"プロジェクトデータがありません"</p>
                    <p class="hint">"右上のメニューから新規作成またはJSONを読み込んでください"</p>
                </div>
            })}
        </div>
    }
}


// JSONファイルをfetch
async fn fetch_json(url: &str) -> Result<ProjectData, String> {
    let opts = RequestInit::new();
    opts.set_method("GET");

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("Request作成失敗: {:?}", e))?;

    let window = web_sys::window().ok_or("windowがありません")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch失敗: {:?}", e))?;

    let resp: Response = resp_value.dyn_into()
        .map_err(|_| "Responseへの変換失敗")?;

    let json = JsFuture::from(resp.json().map_err(|e| format!("json()失敗: {:?}", e))?)
        .await
        .map_err(|e| format!("JSON解析失敗: {:?}", e))?;

    serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("デシリアライズ失敗: {:?}", e))
}

// ============================================
// PDFエディタ
// ============================================

/// Google Drive URLからファイルIDを抽出
fn extract_file_id(url: &str) -> Option<String> {
    // https://drive.google.com/file/d/{FILE_ID}/view
    // https://docs.google.com/document/d/{FILE_ID}/edit
    // https://docs.google.com/spreadsheets/d/{FILE_ID}/edit
    // https://docs.google.com/presentation/d/{FILE_ID}/edit

    if let Some(start) = url.find("/d/") {
        let rest = &url[start + 3..];
        if let Some(end) = rest.find('/') {
            return Some(rest[..end].to_string());
        } else {
            // URLの末尾にファイルIDがある場合
            return Some(rest.to_string());
        }
    }

    // ?id=XXX 形式
    if let Some(start) = url.find("id=") {
        let rest = &url[start + 3..];
        if let Some(end) = rest.find('&') {
            return Some(rest[..end].to_string());
        } else {
            return Some(rest.to_string());
        }
    }

    None
}

#[allow(unused_variables)]
#[component]
fn PdfEditor(
    contractor: String,
    doc_type: String,
    original_url: String,
) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext必須");
    let set_view_mode = ctx.set_view_mode;

    // GAS URLとファイルIDを取得してiframe URLを構築
    let iframe_url = {
        let gas_url = get_gas_url();
        let file_id = extract_file_id(&original_url);

        match (gas_url, file_id) {
            (Some(gas), Some(fid)) => {
                let encoded_gas = js_sys::encode_uri_component(&gas).as_string().unwrap_or_default();
                format!("editor/index.html?fileId={}&gasUrl={}", fid, encoded_gas)
            }
            _ => "editor/index.html".to_string()
        }
    };

    let on_back = move |_| {
        set_view_mode.set(ViewMode::Dashboard);
    };

    // iframeからのpostMessageを受信してダッシュボードに戻る
    create_effect(move |_| {
        let window = web_sys::window().expect("window");
        let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                if let Ok(type_val) = js_sys::Reflect::get(&data, &JsValue::from_str("type")) {
                    if let Some(type_str) = type_val.as_string() {
                        if type_str == "back" {
                            set_view_mode.set(ViewMode::Dashboard);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        window.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref()).ok();
        closure.forget(); // リスナーを保持
    });

    view! {
        <div class="pdf-editor-container">
            <button class="back-btn-float" on:click=on_back>"← 戻る"</button>
            <iframe
                class="pdf-editor-iframe"
                src=iframe_url
                style="width: 100%; height: 100vh; border: none;"
            ></iframe>
        </div>
    }
}

// ============================================
// メインアプリ
// ============================================

// 書類存在チェック実行
fn run_existence_check(project: &ProjectData) -> Vec<CheckResult> {
    let mut results = Vec::new();
    for contractor in &project.contractors {
        for (doc_key, doc_status) in &contractor.docs {
            let label = doc_key.replace("_", " ").chars().skip_while(|c| c.is_numeric()).collect::<String>();
            let label = label.trim_start_matches('_').to_string();

            if !doc_status.status {
                results.push(CheckResult {
                    contractor_name: contractor.name.clone(),
                    doc_name: label,
                    status: CheckStatus::Error,
                    message: doc_status.note.clone().unwrap_or_else(|| "未提出".to_string()),
                });
            } else if doc_status.url.is_none() && doc_status.file.is_some() {
                results.push(CheckResult {
                    contractor_name: contractor.name.clone(),
                    doc_name: label,
                    status: CheckStatus::Warning,
                    message: "URLが未登録".to_string(),
                });
            } else {
                results.push(CheckResult {
                    contractor_name: contractor.name.clone(),
                    doc_name: label,
                    status: CheckStatus::Ok,
                    message: "OK".to_string(),
                });
            }
        }
    }
    results
}

// 日付チェック実行
fn run_date_check(project: &ProjectData, today: &str) -> Vec<CheckResult> {
    let mut results = Vec::new();
    for contractor in &project.contractors {
        for (doc_key, doc_status) in &contractor.docs {
            let label = doc_key.replace("_", " ").chars().skip_while(|c| c.is_numeric()).collect::<String>();
            let label = label.trim_start_matches('_').to_string();

            // 有効期限がある書類のみチェック
            if let Some(ref valid_until) = doc_status.valid_until {
                if valid_until.as_str() < today {
                    results.push(CheckResult {
                        contractor_name: contractor.name.clone(),
                        doc_name: label,
                        status: CheckStatus::Error,
                        message: format!("期限切れ: {}", valid_until),
                    });
                } else {
                    // 30日以内に期限切れになる場合は警告
                    let warning_date = add_days_to_date(today, 30);
                    if valid_until.as_str() <= warning_date.as_str() {
                        results.push(CheckResult {
                            contractor_name: contractor.name.clone(),
                            doc_name: label,
                            status: CheckStatus::Warning,
                            message: format!("期限間近: {}", valid_until),
                        });
                    } else {
                        results.push(CheckResult {
                            contractor_name: contractor.name.clone(),
                            doc_name: label,
                            status: CheckStatus::Ok,
                            message: format!("有効期限: {}", valid_until),
                        });
                    }
                }
            }
        }
    }
    results
}

/// 全書類のチェック結果をクリア
#[allow(dead_code)]
fn clear_all_check_results(project: &mut ProjectData) {
    for contractor in &mut project.contractors {
        for (_, doc) in &mut contractor.docs {
            // DocStatusにはcheck_result, last_checkedフィールドがないため、
            // 将来の拡張用にコメントを残す
            // doc.check_result = None;
            // doc.last_checked = None;
            let _ = doc; // 現在は何もしない
        }
    }
}

/// 特定の書類のチェック結果をクリア
#[allow(dead_code)]
fn clear_check_result(
    project: &mut ProjectData,
    contractor_id: &str,
    doc_key: &str,
) {
    if let Some(contractor) = project.contractors.iter_mut()
        .find(|c| c.id == contractor_id)
    {
        if let Some(doc) = contractor.docs.get_mut(doc_key) {
            // DocStatusにはcheck_result, last_checkedフィールドがないため、
            // 将来の拡張用にコメントを残す
            // doc.check_result = None;
            // doc.last_checked = None;
            let _ = doc; // 現在は何もしない
        }
    }
}

// 日付に日数を加算 (簡易実装)
fn add_days_to_date(date: &str, days: i32) -> String {
    // YYYY-MM-DD形式を想定
    if let Some((year, rest)) = date.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            if let (Ok(y), Ok(m), Ok(d)) = (year.parse::<i32>(), month.parse::<i32>(), day.parse::<i32>()) {
                let total_days = d + days;
                let new_month = m + (total_days - 1) / 30;
                let new_day = ((total_days - 1) % 30) + 1;
                return format!("{:04}-{:02}-{:02}", y + (new_month - 1) / 12, ((new_month - 1) % 12) + 1, new_day);
            }
        }
    }
    date.to_string()
}

// 今日の日付を取得
fn get_today() -> String {
    let date = js_sys::Date::new_0();
    let year = date.get_full_year();
    let month = date.get_month() + 1; // 0-indexed
    let day = date.get_date();
    format!("{:04}-{:02}-{:02}", year, month, day)
}

// タイムスタンプを取得
fn get_timestamp() -> String {
    let date = js_sys::Date::new_0();
    let year = date.get_full_year();
    let month = date.get_month() + 1;
    let day = date.get_date();
    let hours = date.get_hours();
    let minutes = date.get_minutes();
    let seconds = date.get_seconds();
    format!("{:04}{:02}{:02}_{:02}{:02}{:02}", year, month, day, hours, minutes, seconds)
}

// JSONダウンロード用関数（タイムスタンプ付き）
fn download_json(project: &ProjectData) {
    if let Ok(json) = serde_json::to_string_pretty(project) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                // タイムスタンプ付きファイル名
                let timestamp = get_timestamp();
                let project_name = project.project_name.replace(" ", "_").replace("/", "-");
                let filename = format!("{}_{}.json", project_name, timestamp);

                // Blobを作成
                let blob_parts = js_sys::Array::new();
                blob_parts.push(&JsValue::from_str(&json));
                let options = web_sys::BlobPropertyBag::new();
                options.set_type("application/json");

                if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                        if let Ok(a) = document.create_element("a") {
                            let _ = a.set_attribute("href", &url);
                            let _ = a.set_attribute("download", &filename);
                            if let Some(element) = a.dyn_ref::<web_sys::HtmlElement>() {
                                element.click();
                            }
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn App() -> impl IntoView {
    let (menu_open, set_menu_open) = create_signal(false);
    let (copy_success, set_copy_success) = create_signal(false);

    // GAS設定ダイアログ
    let (show_gas_dialog, set_show_gas_dialog) = create_signal(false);
    let (gas_url_input, set_gas_url_input) = create_signal(String::new());
    let (gas_connected, set_gas_connected) = create_signal(get_gas_url().is_some());
    let (gas_syncing, set_gas_syncing) = create_signal(false);
    let (gas_message, set_gas_message) = create_signal(None::<String>);
    let (gas_code, set_gas_code) = create_signal(None::<String>);
    let (gas_code_copied, set_gas_code_copied) = create_signal(false);

    // プロジェクトデータのグローバル状態
    let (project, set_project) = create_signal(None::<ProjectData>);
    let (loading, set_loading) = create_signal(false);
    let (error_msg, set_error_msg) = create_signal(None::<String>);
    let (check_mode, set_check_mode) = create_signal(CheckMode::None);
    let (check_results, set_check_results) = create_signal(Vec::<CheckResult>::new());
    let (edit_mode, set_edit_mode) = create_signal(false);
    let (view_mode, set_view_mode) = create_signal(ViewMode::Dashboard);

    // APIキー設定状態（false = 未設定、ボタン無効化）
    let (api_connected, set_api_connected) = create_signal(false);
    let (api_loading, set_api_loading) = create_signal(false);

    // チェック結果ツールチップ状態
    let (check_result_tooltip, set_check_result_tooltip) = create_signal(CheckResultTooltipState::default());

    // コンテキストメニュー状態（右クリック/ロングプレス）
    let (context_menu, set_context_menu) = create_signal(ContextMenuState::default());

    // データソース追跡（デバッグ用）
    let (data_source, set_data_source) = create_signal("なし".to_string());
    let (show_debug, set_show_debug) = create_signal(false);

    // OCRビュー用の状態
    let (ocr_documents, set_ocr_documents) = create_signal(Vec::<OcrDocument>::new());
    let (current_doc_index, set_current_doc_index) = create_signal(0usize);
    let (selected_token, set_selected_token) = create_signal(None::<usize>);
    let (show_all_boxes, set_show_all_boxes) = create_signal(false);

    // OCRコンテキスト提供
    let ocr_ctx = OcrViewContext {
        documents: ocr_documents,
        set_documents: set_ocr_documents,
        current_doc_index,
        set_current_doc_index,
        selected_token,
        set_selected_token,
        show_all_boxes,
        set_show_all_boxes,
    };
    provide_context(ocr_ctx);

    // コンテキスト提供
    let ctx = ProjectContext {
        project,
        set_project,
        loading,
        set_loading,
        error_msg,
        set_error_msg,
        check_mode,
        set_check_mode,
        check_results,
        set_check_results,
        edit_mode,
        set_edit_mode,
        view_mode,
        set_view_mode,
        api_connected,
        set_api_connected,
        api_loading,
        set_api_loading,
        check_result_tooltip,
        set_check_result_tooltip,
        context_menu,
        set_context_menu,
    };
    provide_context(ctx.clone());

    // iframeからのpostMessageを受信（グローバル）
    {
        let set_view_mode = set_view_mode.clone();
        let set_api_connected = set_api_connected.clone();
        let set_project = set_project.clone();
        let project = project.clone();
        create_effect(move |_| {
            let set_view_mode = set_view_mode.clone();
            let set_api_connected = set_api_connected.clone();
            let set_project = set_project.clone();
            let project = project.clone();
            let window = web_sys::window().expect("window");
            let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                // 全メッセージをログ
                web_sys::console::log_1(&format!("[postMessage] Received event, data type: {:?}", event.data().js_typeof().as_string()).into());
                if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                    if let Ok(type_val) = js_sys::Reflect::get(&data, &JsValue::from_str("type")) {
                        if let Some(type_str) = type_val.as_string() {
                            web_sys::console::log_1(&format!("[postMessage] type={}", type_str).into());
                            match type_str.as_str() {
                                "apikey-setup-complete" => {
                                    // APIキー設定完了 - 状態を更新してダッシュボードに戻る
                                    set_api_connected.set(check_api_key_exists());
                                    set_view_mode.set(ViewMode::Dashboard);

                                    // シート接続中ならAPIキーを自動保存
                                    if let Some(gas_url) = get_gas_url() {
                                        spawn_local(async move {
                                            auto_save_api_key_to_sheet(&gas_url).await;
                                        });
                                    }
                                }
                                "ai-check-result" => {
                                    log_info("ai-check-result", "AIチェック結果を受信");
                                    // AIチェック結果を受け取り、ProjectDataを更新
                                    let contractor = js_sys::Reflect::get(&data, &JsValue::from_str("contractor"))
                                        .ok().and_then(|v| v.as_string());
                                    let doc_key_raw = js_sys::Reflect::get(&data, &JsValue::from_str("docKey"))
                                        .ok().and_then(|v| v.as_string());
                                    let result_val = js_sys::Reflect::get(&data, &JsValue::from_str("result")).ok();
                                    let file_id = js_sys::Reflect::get(&data, &JsValue::from_str("fileId"))
                                        .ok().and_then(|v| v.as_string());

                                    let trace_data = serde_json::json!({
                                        "contractor": contractor,
                                        "doc_key": doc_key_raw,
                                        "file_id": file_id,
                                    });
                                    log_info_with_data("ai-check-result", "受信データ", trace_data);

                                    if let (Some(contractor_name), Some(doc_key_raw), Some(result_js)) = (contractor, doc_key_raw, result_val) {
                                        let doc_key = doc_key_raw.trim().to_string();
                                        // 結果をCheckResultDataにデシリアライズ
                                        match serde_wasm_bindgen::from_value::<CheckResultData>(result_js.clone()) {
                                            Ok(check_result) => {
                                                let trace_data = serde_json::json!({
                                                    "status": check_result.status,
                                                    "extracted_fields": check_result.extracted_fields,
                                                });
                                                log_info_with_data("ai-check-result", "デシリアライズ成功", trace_data);
                                                
                                                // ProjectDataを更新
                                                if let Some(mut proj) = project.get() {
                                                    let now = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();

                                                    // contractor.docsを更新
                                                    let contractor_name_trimmed = contractor_name.trim();
                                                    let mut updated = false;
                                                    for contractor in &mut proj.contractors {
                                                        if contractor.name.trim() == contractor_name_trimmed {
                                                            log_info("ai-check-result", &format!("業者を発見: {}", contractor.name));
                                                            
                                                            let doc_keys: Vec<String> = contractor.docs.keys().cloned().collect();
                                                            let trace_data = serde_json::json!({
                                                                "available_doc_keys": doc_keys,
                                                                "target_doc_key": doc_key,
                                                            });
                                                            log_info_with_data("ai-check-result", "利用可能なdocキー", trace_data);
                                                            
                                                            if let Some(doc_status) = contractor.docs.get_mut(&doc_key) {
                                                                // 既存のcheck_resultをログ
                                                                if let Some(ref old_result) = doc_status.check_result {
                                                                    let trace_data = serde_json::json!({
                                                                        "old_status": old_result.status,
                                                                        "old_extracted_fields": old_result.extracted_fields,
                                                                    });
                                                                    log_info_with_data("ai-check-result", "既存のcheck_result", trace_data);
                                                                }
                                                                doc_status.check_result = Some(check_result.clone());
                                                                doc_status.last_checked = Some(now.clone());
                                                                updated = true;
                                                                
                                                                let trace_data = serde_json::json!({
                                                                    "doc_key": doc_key,
                                                                    "new_extracted_fields": check_result.extracted_fields,
                                                                    "last_checked": now,
                                                                });
                                                                log_info_with_data("ai-check-result", "ドキュメント更新完了", trace_data);
                                                            } else {
                                                                let trace_data = serde_json::json!({
                                                                    "doc_key": doc_key,
                                                                    "available_keys": doc_keys,
                                                                });
                                                                log_error_with_data("ai-check-result", &format!("Doc key '{}' が見つかりません", doc_key), trace_data);
                                                            }
                                                            break;
                                                        }
                                                    }
                                                    if !updated {
                                                        let trace_data = serde_json::json!({
                                                            "contractor_name": contractor_name,
                                                        });
                                                        log_error_with_data("ai-check-result", &format!("業者 '{}' が見つかりません", contractor_name), trace_data);
                                                    }

                                                    // ローカル更新
                                                    set_project.set(Some(proj.clone()));
                                                    save_to_cache(&proj);
                                                    log_info("ai-check-result", "キャッシュに保存完了");

                                                    // GASに保存
                                                    let proj_for_sync = proj.clone();
                                                    spawn_local(async move {
                                                        match sync_to_gas(&proj_for_sync).await {
                                                            Ok(msg) => {
                                                                log_info("gas-sync", &format!("GAS保存成功: {}", msg));
                                                            }
                                                            Err(e) => {
                                                                log_error("gas-sync", &format!("GAS保存エラー: {}", e));
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                            Err(e) => {
                                                let trace_data = serde_json::json!({
                                                    "error": format!("{:?}", e),
                                                });
                                                log_error_with_data("ai-check-result", "デシリアライズエラー", trace_data);
                                            }
                                        }
                                    } else {
                                        log_error("ai-check-result", "必要なパラメータが不足しています");
                                    }
                                    // チェック結果パネルをクリア
                                    set_check_mode.set(CheckMode::None);
                                    set_check_results.set(Vec::new());
                                    set_view_mode.set(ViewMode::Dashboard);
                                }
                                "ai-check-cancel" | "back" => {
                                    // チェック結果パネルをクリア
                                    set_check_mode.set(CheckMode::None);
                                    set_check_results.set(Vec::new());
                                    set_view_mode.set(ViewMode::Dashboard);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);
            window.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref()).ok();
            closure.forget();
        });
    }

    // 起動時にAPIキー設定をチェック
    set_api_connected.set(check_api_key_exists());

    // GAS URLパラメータ初期化 (?gas=xxx) または保存済みGAS URLからデータ取得
    let gas_source = if init_gas_from_url_params().is_some() {
        Some("GAS (URLパラメータ)")
    } else if get_gas_url().is_some() {
        Some("GAS (保存済みURL)")
    } else {
        None
    };

    if let Some(source) = gas_source {
        set_gas_connected.set(true);
        let source_str = source.to_string();
        // GASからデータを取得
        spawn_local(async move {
            set_gas_syncing.set(true);
            match fetch_from_gas().await {
                Ok(data) => {
                    set_project.set(Some(data.clone()));
                    save_to_cache(&data);
                    set_data_source.set(source_str);
                    set_gas_message.set(Some("シートからデータを読み込みました".to_string()));
                }
                Err(e) => {
                    set_gas_message.set(Some(format!("読み込みエラー: {}", e)));
                }
            }
            set_gas_syncing.set(false);
        });
    }

    // 初期読み込み: URLハッシュ → キャッシュ の順で試行
    create_effect(move |_| {
        if project.get().is_none() {
            if let Some(data) = get_hash_data() {
                set_project.set(Some(data.clone()));
                save_to_cache(&data);
                set_data_source.set("URLハッシュ".to_string());
            } else if let Some(data) = load_from_cache() {
                set_project.set(Some(data));
                set_data_source.set("LocalStorageキャッシュ".to_string());
            }
        }
    });

    // プロジェクトが更新されたらキャッシュに保存
    create_effect(move |_| {
        if let Some(p) = project.get() {
            save_to_cache(&p);
        }
    });

    // JSONファイル読み込み
    let on_file_change = move |ev: web_sys::Event| {
        let input: HtmlInputElement = event_target(&ev);
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();

                let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Ok(result) = reader_clone.result() {
                        if let Some(text) = result.as_string() {
                            match serde_json::from_str::<ProjectData>(&text) {
                                Ok(data) => {
                                    set_project.set(Some(data));
                                    set_error_msg.set(None);
                                }
                                Err(e) => {
                                    set_error_msg.set(Some(format!("JSON解析エラー: {}", e)));
                                }
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);

                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                let _ = reader.read_as_text(&file);
            }
        }
        set_menu_open.set(false);
    };

    // サンプルデータ読み込み
    let load_sample = move |_| {
        set_menu_open.set(false);
        spawn_local(async move {
            set_loading.set(true);
            match fetch_json("data/sample_project.json").await {
                Ok(data) => {
                    set_project.set(Some(data));
                    set_error_msg.set(None);
                }
                Err(e) => {
                    set_error_msg.set(Some(e));
                }
            }
            set_loading.set(false);
        });
    };

    // 共有URL生成
    let generate_share_url = move |_| {
        if let Some(p) = project.get() {
            let json = serde_json::to_string(&p).ok();
            if let Some(json_str) = json {
                if let Some(encoded) = encode_base64(&json_str) {
                    let hash = format!("#data={}", encoded);
                    if let Some(window) = web_sys::window() {
                        // アドレスバーのハッシュを更新
                        let _ = window.location().set_hash(&hash);

                        // フルURLを取得してクリップボードにコピー
                        if let Ok(href) = window.location().href() {
                            let clipboard = window.navigator().clipboard();
                            let promise = clipboard.write_text(&href);

                            // 非同期でクリップボードにコピー
                            spawn_local(async move {
                                match JsFuture::from(promise).await {
                                    Ok(_) => {
                                        set_copy_success.set(true);
                                        // コンソールにも出力
                                        web_sys::console::log_1(&"共有URLをクリップボードにコピーしました".into());
                                        gloo::timers::future::TimeoutFuture::new(3000).await;
                                        set_copy_success.set(false);
                                    }
                                    Err(e) => {
                                        web_sys::console::error_1(&format!("クリップボードへのコピー失敗: {:?}", e).into());
                                        // フォールバック: alertで表示
                                        if let Some(window) = web_sys::window() {
                                            let _ = window.alert_with_message(&format!("共有URL:\n{}", href));
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
        set_menu_open.set(false);
    };

    // キャッシュクリア
    let on_clear_cache = move |_| {
        clear_cache();
        set_project.set(None);
        set_check_mode.set(CheckMode::None);
        set_check_results.set(Vec::new());
        set_menu_open.set(false);
    };

    // 書類存在チェック
    let on_existence_check = move |_| {
        set_menu_open.set(false);
        if let Some(p) = project.get() {
            let results = run_existence_check(&p);
            set_check_results.set(results);
            set_check_mode.set(CheckMode::Existence);
        }
    };

    // 日付チェック
    let on_date_check = move |_| {
        set_menu_open.set(false);
        if let Some(p) = project.get() {
            let today = get_today();
            let results = run_date_check(&p, &today);
            set_check_results.set(results);
            set_check_mode.set(CheckMode::Date);
        }
    };

    // 新規プロジェクト作成
    let on_new_project = move |_| {
        set_menu_open.set(false);
        let new_project = ProjectData {
            project_name: "新規工事".to_string(),
            client: "".to_string(),
            period: "".to_string(),
            period_start: None,
            period_end: None,
            site_representative: None,
            chief_engineer: None,
            project_docs: ProjectDocs::default(),
            contractors: vec![
                Contractor {
                    id: "prime".to_string(),
                    name: "元請業者".to_string(),
                    role: "元請".to_string(),
                    docs: HashMap::new(),
                }
            ],
            contracts: Vec::new(),
        };
        set_project.set(Some(new_project));
        set_edit_mode.set(true);
    };

    // 編集モード切り替え
    let toggle_edit_mode = move |_| {
        set_menu_open.set(false);
        set_edit_mode.update(|e| *e = !*e);
    };

    // JSONエクスポート
    let on_export_json = move |_| {
        set_menu_open.set(false);
        if let Some(p) = project.get() {
            download_json(&p);
        }
    };

    view! {
        <div class="app">
            <header class="app-header">
                <div class="menu-container">
                    <button class="menu-btn" on:click=move |_| set_menu_open.update(|v| *v = !*v)>
                        "⋮"
                    </button>
                    {move || menu_open.get().then(|| view! {
                        <div class="menu-dropdown">
                            <button class="menu-item" on:click=on_new_project>
                                "新規作成"
                            </button>
                            <label class="menu-item file-input-label">
                                "JSONを読み込む"
                                <input type="file" accept=".json" on:change=on_file_change style="display:none" />
                            </label>
                            <button class="menu-item" on:click=load_sample disabled=move || loading.get()>
                                {move || if loading.get() { "読込中..." } else { "サンプル読込" }}
                            </button>
                            <hr class="menu-divider" />
                            <button class="menu-item" on:click=toggle_edit_mode disabled=move || project.get().is_none()>
                                {move || if edit_mode.get() { "編集を終了" } else { "編集モード" }}
                            </button>
                            <hr class="menu-divider" />
                            <button class="menu-item" on:click=on_existence_check disabled=move || project.get().is_none() || edit_mode.get()>
                                "書類存在チェック"
                            </button>
                            <button class="menu-item" on:click=on_date_check disabled=move || project.get().is_none() || edit_mode.get()>
                                "日付チェック"
                            </button>
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                set_check_mode.set(CheckMode::None);
                                set_check_results.set(Vec::new());
                            } disabled=move || check_mode.get() == CheckMode::None>
                                "チェック結果をクリア"
                            </button>
                            <hr class="menu-divider" />
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                set_view_mode.set(if view_mode.get() == ViewMode::OcrViewer {
                                    ViewMode::Dashboard
                                } else {
                                    ViewMode::OcrViewer
                                });
                            }>
                                {move || if view_mode.get() == ViewMode::OcrViewer {
                                    "ダッシュボードに戻る"
                                } else {
                                    "OCR座標表示"
                                }}
                            </button>
                            <label class="menu-item file-input-label">
                                "OCRトークンJSON読込"
                                <input type="file" accept=".json" on:change=move |ev: web_sys::Event| {
                                    let input: HtmlInputElement = event_target(&ev);
                                    if let Some(files) = input.files() {
                                        if let Some(file) = files.get(0) {
                                            let reader = FileReader::new().unwrap();
                                            let reader_clone = reader.clone();
                                            let filename = file.name();

                                            let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                                                if let Ok(result) = reader_clone.result() {
                                                    if let Some(text) = result.as_string() {
                                                        match serde_json::from_str::<Vec<OcrToken>>(&text) {
                                                            Ok(tokens) => {
                                                                let doc = OcrDocument {
                                                                    contractor: filename.replace(".json", "").replace("debug_tokens_", ""),
                                                                    doc_type: "OCR読込".to_string(),
                                                                    image_url: String::new(),
                                                                    tokens,
                                                                };
                                                                set_ocr_documents.update(|docs| docs.push(doc));
                                                                set_view_mode.set(ViewMode::OcrViewer);
                                                            }
                                                            Err(e) => {
                                                                web_sys::console::log_1(&format!("OCR JSON解析エラー: {}", e).into());
                                                            }
                                                        }
                                                    }
                                                }
                                            }) as Box<dyn FnMut(_)>);

                                            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                                            onload.forget();
                                            let _ = reader.read_as_text(&file);
                                        }
                                    }
                                    set_menu_open.set(false);
                                } style="display:none" />
                            </label>
                            <hr class="menu-divider" />
                            <button class="menu-item" on:click=on_export_json disabled=move || project.get().is_none()>
                                "JSONエクスポート"
                            </button>
                            <button class="menu-item" on:click=generate_share_url disabled=move || project.get().is_none()>
                                {move || if copy_success.get() { "URLをコピーしました!" } else { "共有URLを生成" }}
                            </button>
                            <hr class="menu-divider" />
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                set_view_mode.set(ViewMode::ApiKeySetup);
                            }>
                                "APIキー設定"
                            </button>
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                if let Some(window) = web_sys::window() {
                                    let _ = window.open_with_url_and_target("/health-report.html", "_blank");
                                }
                            }>
                                "健全性ダッシュボード"
                            </button>
                            <hr class="menu-divider" />
                            // GAS連携
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                set_gas_url_input.set(get_gas_url().unwrap_or_default());
                                set_show_gas_dialog.set(true);
                            }>
                                {move || if gas_connected.get() { "シート設定 (接続中)" } else { "シート連携設定" }}
                            </button>
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                if let Some(url) = generate_gas_share_url() {
                                    if let Some(window) = web_sys::window() {
                                        let clipboard = window.navigator().clipboard();
                                        let promise = clipboard.write_text(&url);
                                        spawn_local(async move {
                                            match JsFuture::from(promise).await {
                                                Ok(_) => {
                                                    set_gas_message.set(Some("共有URLをコピーしました".to_string()));
                                                }
                                                Err(_) => {
                                                    let _ = window.alert_with_message(&format!("共有URL:\n{}", url));
                                                }
                                            }
                                        });
                                    }
                                } else {
                                    set_gas_message.set(Some("シート連携が未設定です".to_string()));
                                }
                            } disabled=move || !gas_connected.get()>
                                "シート共有URLをコピー"
                            </button>
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                if project.get().is_some() {
                                    spawn_local(async move {
                                        set_gas_syncing.set(true);
                                        let p = project.get().unwrap();
                                        match sync_to_gas(&p).await {
                                            Ok(ts) => {
                                                set_gas_message.set(Some(format!("保存完了: {}", ts)));
                                            }
                                            Err(e) => {
                                                set_gas_message.set(Some(format!("保存エラー: {}", e)));
                                            }
                                        }
                                        set_gas_syncing.set(false);
                                    });
                                }
                            } disabled=move || !gas_connected.get() || project.get().is_none() || gas_syncing.get()>
                                {move || if gas_syncing.get() { "保存中..." } else { "シートに保存" }}
                            </button>
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                spawn_local(async move {
                                    set_gas_syncing.set(true);
                                    match fetch_from_gas().await {
                                        Ok(data) => {
                                            set_project.set(Some(data.clone()));
                                            save_to_cache(&data);
                                            set_gas_message.set(Some("シートからデータを読み込みました".to_string()));
                                        }
                                        Err(e) => {
                                            set_gas_message.set(Some(format!("読み込みエラー: {}", e)));
                                        }
                                    }
                                    set_gas_syncing.set(false);
                                });
                            } disabled=move || !gas_connected.get() || gas_syncing.get()>
                                {move || if gas_syncing.get() { "読み込み中..." } else { "シートから読込" }}
                            </button>
                            <hr class="menu-divider" />
                            <a class="menu-item" href="https://github.com/YuujiKamura/SekouTaiseiMaker" target="_blank" rel="noopener">
                                "GitHub リポジトリ ↗"
                            </a>
                            <a class="menu-item" href="https://github.com/YuujiKamura/SekouTaiseiMaker/actions" target="_blank" rel="noopener">
                                "GitHub Actions ↗"
                            </a>
                            <hr class="menu-divider" />
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                spawn_local(async move {
                                    match copy_logs_to_clipboard_async().await {
                                        Ok(_) => {
                                            if let Some(window) = web_sys::window() {
                                                let _ = window.alert_with_message("ログをクリップボードにコピーしました");
                                            }
                                        }
                                        Err(e) => {
                                            if let Some(window) = web_sys::window() {
                                                let _ = window.alert_with_message(&format!("コピー失敗: {}", e));
                                            }
                                        }
                                    }
                                });
                            }>
                                "ログをクリップボードにコピー"
                            </button>
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                download_logs();
                            }>
                                "ログダウンロード"
                            </button>
                            <button class="menu-item" on:click=move |_| {
                                set_menu_open.set(false);
                                clear_logs();
                            }>
                                "ログクリア"
                            </button>
                            <button class="menu-item" on:click=move |_| set_show_debug.update(|v| *v = !*v)>
                                {move || if show_debug.get() { "デバッグ非表示" } else { "デバッグ表示" }}
                            </button>
                            <button class="menu-item danger" on:click=on_clear_cache>
                                "キャッシュクリア"
                            </button>
                        </div>
                    })}
                </div>
            </header>

            // デバッグパネル
            <Show when=move || show_debug.get() fallback=|| ()>
                <div class="debug-panel">
                    <div class="debug-header">"データソース情報"</div>
                    <div class="debug-content">
                        <p><strong>"ソース: "</strong>{move || data_source.get()}</p>
                        <p><strong>"GAS URL: "</strong>{move || get_gas_url().unwrap_or_else(|| "未設定".to_string())}</p>
                        {move || project.get().map(|p| view! {
                            <div class="debug-project">
                                <p><strong>"プロジェクト: "</strong>{p.project_name.clone()}</p>
                                <p><strong>"業者数: "</strong>{p.contractors.len()}</p>
                                <div class="debug-contractors">
                                    {p.contractors.iter().map(|c| {
                                        let name = c.name.clone();
                                        let docs: Vec<_> = c.docs.iter().map(|(k, v)| {
                                            let key = k.clone();
                                            let url = v.url.clone().unwrap_or_else(|| "なし".to_string());
                                            let status = v.status.clone();
                                            view! {
                                                <li>
                                                    <span class="debug-key">{key}</span>
                                                    <span class="debug-status">" ["{status}"]"</span>
                                                    <br/>
                                                    <span class="debug-url">{url}</span>
                                                </li>
                                            }
                                        }).collect();
                                        view! {
                                            <details class="debug-contractor">
                                                <summary>{name}" ("{c.docs.len()}"件)"</summary>
                                                <ul class="debug-urls">{docs}</ul>
                                            </details>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        })}
                    </div>
                </div>
            </Show>

            {move || {
                match view_mode.get() {
                    ViewMode::Dashboard => view! {
                        <main class="container">
                            <Dashboard />
                            <CheckResultsPanel />
                            <CheckResultTooltip />
                            <ContextMenu />
                        </main>
                    }.into_view(),

                    ViewMode::OcrViewer => view! {
                        <main class="container">
                            <OcrViewer />
                        </main>
                    }.into_view(),

                    ViewMode::PdfViewer { contractor, doc_type, url, doc_key, contractor_id } => view! {
                        <PdfViewer
                            contractor=contractor
                            doc_type=doc_type
                            url=url
                            doc_key=doc_key
                            contractor_id=contractor_id
                        />
                    }.into_view(),

                    ViewMode::SpreadsheetViewer { contractor, doc_type, url, doc_key, contractor_id } => view! {
                        <SpreadsheetViewer
                            contractor=contractor
                            doc_type=doc_type
                            url=url
                            doc_key=doc_key
                            contractor_id=contractor_id
                        />
                    }.into_view(),

                    ViewMode::PdfEditor { contractor, doc_type, original_url } => view! {
                        <PdfEditor
                            contractor=contractor
                            doc_type=doc_type
                            original_url=original_url
                        />
                    }.into_view(),

                    ViewMode::AiChecker { contractor, doc_type, file_id, doc_key, contractor_id } => {
                        let gas_url = get_gas_url().unwrap_or_default();
                        let iframe_url = format!(
                            "editor/index.html?mode=check&fileId={}&docType={}&contractor={}&gasUrl={}&docKey={}&contractorId={}",
                            file_id,
                            js_sys::encode_uri_component(&doc_type),
                            js_sys::encode_uri_component(&contractor),
                            js_sys::encode_uri_component(&gas_url),
                            js_sys::encode_uri_component(&doc_key),
                            js_sys::encode_uri_component(&contractor_id)
                        );
                        view! {
                            <iframe src=iframe_url style="width: 100%; height: 100vh; border: none;"></iframe>
                        }.into_view()
                    },

                    ViewMode::ApiKeySetup => view! {
                        <div class="api-key-setup-container">
                            <div class="back-button-container">
                                <button class="back-btn" on:click=move |_| set_view_mode.set(ViewMode::Dashboard)>
                                    "← 戻る"
                                </button>
                            </div>
                            <iframe src="editor/index.html?mode=apikey" style="width: 100%; height: calc(100vh - 50px); border: none;"></iframe>
                        </div>
                    }.into_view(),
                }
            }}

            // GASメッセージ通知
            {move || gas_message.get().map(|msg| view! {
                <div class="gas-toast" on:click=move |_| set_gas_message.set(None)>
                    {msg}
                </div>
            })}

            // GAS設定ダイアログ
            {move || show_gas_dialog.get().then(|| view! {
                <div class="gas-dialog-overlay" on:click=move |_| set_show_gas_dialog.set(false)>
                    <div class="gas-dialog" on:click=move |e| e.stop_propagation()>
                        <div class="gas-dialog-header">
                            <h3>"シート連携設定"</h3>
                            <span class="gas-script-modified">{format_gas_modified_time()}</span>
                            <button class="close-btn" on:click=move |_| set_show_gas_dialog.set(false)>"×"</button>
                        </div>
                        <div class="gas-dialog-body">
                            <div class="gas-step">
                                <span class="step-num">"1"</span>
                                <div class="step-content">
                                    <p class="step-title">"Google スプレッドシートを作成"</p>
                                    <p class="step-desc">"拡張機能 → Apps Script を開く"</p>
                                </div>
                            </div>
                            <div class="gas-step">
                                <span class="step-num">"2"</span>
                                <div class="step-content">
                                    <p class="step-title">"GASコードを貼り付け"</p>
                                    <div class="gas-code-actions">
                                        <button
                                            class="gas-btn"
                                            on:click=move |_| {
                                                set_gas_code.set(Some(include_str!("../gas/SekouTaiseiSync.gs").to_string()));
                                                set_gas_code_copied.set(false);
                                            }
                                        >
                                            "GASコードを表示"
                                        </button>
                                        {move || gas_code.get().map(|code| {
                                            let code_for_copy = code.clone();
                                            view! {
                                                <button
                                                    class=move || if gas_code_copied.get() { "gas-btn copied" } else { "gas-btn primary" }
                                                    on:click=move |_| {
                                                        if let Some(window) = web_sys::window() {
                                                            let clipboard = window.navigator().clipboard();
                                                            let promise = clipboard.write_text(&code_for_copy);
                                                            spawn_local(async move {
                                                                if JsFuture::from(promise).await.is_ok() {
                                                                    set_gas_code_copied.set(true);
                                                                }
                                                            });
                                                        }
                                                    }
                                                >
                                                    {move || if gas_code_copied.get() { "コピーしました!" } else { "コードをコピー" }}
                                                </button>
                                            }
                                        })}
                                    </div>
                                    {move || gas_code.get().map(|code| view! {
                                        <textarea
                                            class="gas-code-display"
                                            readonly=true
                                            rows="8"
                                        >{code}</textarea>
                                    })}
                                </div>
                            </div>
                            <div class="gas-step">
                                <span class="step-num">"3"</span>
                                <div class="step-content">
                                    <p class="step-title">"ウェブアプリとしてデプロイ"</p>
                                    <p class="step-desc">"アクセス: 全員 → デプロイ → URLをコピー"</p>
                                </div>
                            </div>
                            <div class="gas-step">
                                <span class="step-num">"4"</span>
                                <div class="step-content">
                                    <p class="step-title">"URLを貼り付け"</p>
                                    <input
                                        type="text"
                                        class="gas-url-input"
                                        placeholder="https://script.google.com/macros/s/..."
                                        prop:value=move || gas_url_input.get()
                                        on:input=move |ev| {
                                            let value = event_target_value(&ev);
                                            set_gas_url_input.set(value);
                                        }
                                    />
                                </div>
                            </div>
                        </div>
                        <div class="gas-dialog-footer">
                            {move || gas_connected.get().then(|| view! {
                                <button class="gas-btn danger" on:click=move |_| {
                                    clear_gas_url();
                                    set_gas_connected.set(false);
                                    set_gas_url_input.set(String::new());
                                    set_gas_message.set(Some("連携を解除しました".to_string()));
                                }>"連携解除"</button>
                            })}
                            <button class="gas-btn primary" on:click=move |_| {
                                let url = gas_url_input.get();
                                if !url.is_empty() && url.starts_with("https://script.google.com/") {
                                    save_gas_url(&url);
                                    set_gas_connected.set(true);
                                    set_show_gas_dialog.set(false);

                                    let url_clone = url.clone();

                                    // プロジェクトデータがあれば自動保存
                                    if let Some(p) = project.get() {
                                        spawn_local(async move {
                                            set_gas_syncing.set(true);
                                            match sync_to_gas(&p).await {
                                                Ok(_) => {
                                                    set_gas_message.set(Some("シート連携を設定し、データを保存しました".to_string()));
                                                }
                                                Err(e) => {
                                                    set_gas_message.set(Some(format!("連携設定完了、保存エラー: {}", e)));
                                                }
                                            }
                                            // APIキーとGAS URL自体も自動保存
                                            auto_save_api_key_to_sheet(&url_clone).await;
                                            let _ = save_gas_url_to_sheet(&url_clone).await;
                                            set_gas_syncing.set(false);
                                        });
                                    } else {
                                        // プロジェクトデータがなくてもAPIキーとGAS URLは保存
                                        spawn_local(async move {
                                            auto_save_api_key_to_sheet(&url_clone).await;
                                            let _ = save_gas_url_to_sheet(&url_clone).await;
                                        });
                                        set_gas_message.set(Some("シート連携を設定しました".to_string()));
                                    }
                                } else {
                                    set_gas_message.set(Some("正しいGAS URLを入力してください".to_string()));
                                }
                            }>"保存"</button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}


fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

