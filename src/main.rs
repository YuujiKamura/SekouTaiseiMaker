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
use web_sys::{FileReader, HtmlInputElement, Request, RequestInit, Response, HtmlCanvasElement, CanvasRenderingContext2d, HtmlImageElement};
use std::collections::HashMap;

// 自モジュールからのインポート
use models::*;
use utils::cache::{save_to_cache, load_from_cache, clear_cache};
use utils::gas::{get_gas_url, save_gas_url, clear_gas_url, init_gas_from_url_params, generate_gas_share_url, fetch_from_gas, save_to_gas, auto_save_api_key_to_sheet};
use utils::{encode_base64, decode_base64};
use views::PdfViewer;


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

// URLハッシュにデータを設定
fn set_hash_data(project: &ProjectData) -> Option<String> {
    let json = serde_json::to_string(project).ok()?;
    let encoded = encode_base64(&json)?;
    let window = web_sys::window()?;
    let location = window.location();
    let base_url = format!(
        "{}//{}{}",
        location.protocol().ok()?,
        location.host().ok()?,
        location.pathname().ok()?
    );
    let share_url = format!("{}#data={}", base_url, encoded);
    Some(share_url)
}


/// GASにプロジェクトデータを保存
async fn sync_to_gas(project: &ProjectData) -> Result<String, String> {
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

// ContextMenuStateはmodels::TooltipStateとしてエクスポート済み
// main.rsでは互換性のためエイリアスを定義
pub type ContextMenuState = models::TooltipState;

// ============================================
// Google Drive/Sheets URL解析ヘルパー関数
// ============================================

/// Google DriveファイルIDからプレビューURLを構築
fn build_drive_preview_url(file_id: &str) -> String {
    format!("https://drive.google.com/file/d/{}/preview", file_id)
}

/// Google Sheets URLからスプレッドシートIDを抽出
/// パターン: /spreadsheets/d/{SPREADSHEET_ID}/...
fn extract_spreadsheet_id(url: &str) -> Option<String> {
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

/// Google Sheets URLからスプレッドシートIDとgidを抽出
fn extract_spreadsheet_info(url: &str) -> Option<(String, Option<String>)> {
    extract_spreadsheet_id(url).map(|id| (id, extract_gid(url)))
}

/// Google Sheets埋め込みURLを構築
fn build_sheets_embed_url(spreadsheet_id: &str, gid: Option<&str>) -> String {
    match gid {
        Some(g) => format!(
            "https://docs.google.com/spreadsheets/d/{}/preview?gid={}",
            spreadsheet_id, g
        ),
        None => format!(
            "https://docs.google.com/spreadsheets/d/{}/preview",
            spreadsheet_id
        ),
    }
}

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

/// OCR結果から不足フィールドを検出
fn detect_missing_fields(ocr_result: &OcrResult) -> Vec<MissingField> {
    let mut missing = Vec::new();

    // 日付フィールドのチェック
    if !ocr_result.text.contains("令和") {
        missing.push(MissingField {
            field_name: "日付".to_string(),
            field_type: FieldType::Date,
            value: String::new(),
            position: None,
        });
    }

    // 署名フィールドのチェック
    if !ocr_result.text.contains("印") {
        missing.push(MissingField {
            field_name: "代表者印".to_string(),
            field_type: FieldType::Signature,
            value: String::new(),
            position: None,
        });
    }

    // 会社名フィールドのチェック
    if !ocr_result.text.contains("株式会社") && !ocr_result.text.contains("有限会社") {
        missing.push(MissingField {
            field_name: "会社名".to_string(),
            field_type: FieldType::Text,
            value: String::new(),
            position: None,
        });
    }

    missing
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
    /// コンテキストメニュー状態
    pub context_menu: ReadSignal<ContextMenuState>,
    pub set_context_menu: WriteSignal<ContextMenuState>,
}

// 標準的な書類リスト
const STANDARD_DOCS: &[(&str, &str)] = &[
    ("01_建設業許可", "建設業許可"),
    ("02_事業所番号", "事業所番号"),
    ("03_労働保険番号", "労働保険番号"),
    ("041_現場代理人資格", "現場代理人資格"),
    ("042_現場代理人在籍", "現場代理人在籍"),
    ("051_主任技術者資格", "主任技術者資格"),
    ("052_主任技術者在籍", "主任技術者在籍"),
    ("06_法定外労災", "法定外労災"),
    ("07_建退共", "建退共"),
    ("08_作業員名簿", "作業員名簿"),
    ("09_暴対法誓約書", "暴対法誓約書"),
];

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

/// ホバーツールチップ（1秒ホバーで表示）
#[component]
fn ContextMenu() -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");
    let menu_state = ctx.context_menu;

    view! {
        {move || {
            let state = menu_state.get();
            if !state.visible {
                return view! { <></> }.into_view();
            }

            let status_text = state.check_result.as_ref().map(|r| {
                match r.status.as_str() {
                    "ok" => ("✓ OK", "status-ok"),
                    "warning" => ("⚠ 要確認", "status-warning"),
                    "error" => ("! 要対応", "status-error"),
                    _ => ("? 不明", "status-unknown"),
                }
            });

            let summary = state.check_result.as_ref().map(|r| r.summary.clone());
            let items = state.check_result.as_ref().map(|r| r.items.clone()).unwrap_or_default();
            let last_checked = state.last_checked.clone();

            // 画面内に収まるように位置調整
            let window = web_sys::window().unwrap();
            let vw = window.inner_width().unwrap().as_f64().unwrap_or(800.0) as i32;
            let vh = window.inner_height().unwrap().as_f64().unwrap_or(600.0) as i32;
            let menu_width = 320;  // 推定メニュー幅
            let menu_height = 300; // 推定メニュー高さ

            let x = if state.x + menu_width > vw {
                (state.x - menu_width).max(0)
            } else {
                state.x
            };
            let y = if state.y + menu_height > vh {
                (state.y - menu_height).max(0)
            } else {
                state.y
            };

            view! {
                <div
                    class="hover-tooltip"
                    style=format!("left: {}px; top: {}px;", x, y)
                >
                    <div class="tooltip-header">
                        <span class="contractor-name">{state.contractor_name.clone()}</span>
                        <span class="doc-label">{state.doc_label.clone()}</span>
                    </div>

                    {match &state.check_result {
                        Some(_) => view! {
                            <div class="tooltip-content">
                                <div class=format!("status-line {}", status_text.map(|(_, c)| c).unwrap_or(""))>
                                    {status_text.map(|(t, _)| t).unwrap_or("未チェック")}
                                </div>

                                {summary.filter(|s| !s.is_empty()).map(|s| view! {
                                    <div class="summary">{s}</div>
                                })}

                                {(!items.is_empty()).then(|| view! {
                                    <div class="issues">
                                        <span class="issues-title">"チェック項目:"</span>
                                        <ul>
                                            {items.iter().map(|item: &CheckItem| view! {
                                                <li class=format!("item-{}", item.item_type)>{item.message.clone()}</li>
                                            }).collect_view()}
                                        </ul>
                                    </div>
                                })}

                                {last_checked.map(|dt| view! {
                                    <div class="checked-at">"チェック日時: " {dt}</div>
                                })}
                            </div>
                        }.into_view(),
                        None => view! {
                            <div class="tooltip-content no-result">
                                "AIチェック未実施"
                            </div>
                        }.into_view(),
                    }}
                </div>
            }.into_view()
        }}
    }
}

#[component]
fn ProjectView(project: ProjectData) -> impl IntoView {
    let total_docs: usize = project.contractors.iter().map(|c| c.docs.len()).sum();
    let complete_docs: usize = project.contractors.iter()
        .flat_map(|c| c.docs.values())
        .filter(|d| d.status)
        .count();
    let progress = if total_docs > 0 { (complete_docs * 100) / total_docs } else { 0 };

    let project_docs = project.project_docs.clone();

    view! {
        <div class="project-view">
            <div class="project-header">
                <h3>{project.project_name.clone()}</h3>
                <div class="project-meta">
                    <span class="client">{project.client.clone()}</span>
                    <span class="period">{project.period.clone()}</span>
                </div>
            </div>

            <div class="progress-section">
                <div class="progress-bar">
                    <div class="progress-fill" style=format!("width: {}%", progress)></div>
                </div>
                <span class="progress-text">{complete_docs}"/" {total_docs} " (" {progress}"%)"</span>
            </div>

            // 全体書類セクション
            <div class="project-docs-section">
                <h4>"全体書類"</h4>
                <div class="project-docs-grid">
                    <ProjectDocCard
                        label="施工体系図"
                        doc=project_docs.sekou_taikeizu.clone()
                    />
                    <ProjectDocCard
                        label="施工体制台帳"
                        doc=project_docs.sekou_taisei_daicho.clone()
                    />
                    <ProjectDocCard
                        label="下請契約書"
                        doc=project_docs.shitauke_keiyaku.clone()
                    />
                </div>
            </div>

            // 各社書類セクション
            <div class="contractors-section">
                <h4>"各社書類"</h4>
                <div class="contractors-grid">
                    {project.contractors.into_iter().map(|c| view! {
                        <ContractorCard contractor=c />
                    }).collect_view()}
                </div>
            </div>

            // 下請施工体制セクション
            {(!project.contracts.is_empty()).then(|| view! {
                <div class="contracts-section">
                    <h4>"下請施工体制"</h4>
                    <div class="contracts-list">
                        {project.contracts.into_iter().map(|c| view! {
                            <div class="contract-item">
                                {if let Some(url) = c.url {
                                    view! {
                                        <a class="contract-link" href=url target="_blank" rel="noopener">{c.name}</a>
                                    }.into_view()
                                } else {
                                    view! {
                                        <span class="contract-name">{c.name}</span>
                                    }.into_view()
                                }}
                                {c.contractor.map(|contractor| view! {
                                    <span class="contract-contractor">{contractor}</span>
                                })}
                            </div>
                        }).collect_view()}
                    </div>
                </div>
            })}
        </div>
    }
}

#[component]
fn ProjectDocCard(label: &'static str, doc: Option<DocLink>) -> impl IntoView {
    let (has_doc, url, status) = match &doc {
        Some(d) => (true, d.url.clone(), d.status),
        None => (false, None, false),
    };

    view! {
        <div class=format!("project-doc-card {}", if status { "complete" } else if has_doc { "incomplete" } else { "empty" })>
            <span class="doc-icon">{
                if status { "✓" } else if has_doc { "○" } else { "−" }
            }</span>
            {if let Some(u) = url {
                view! {
                    <a class="doc-link" href=u target="_blank" rel="noopener">{label}</a>
                }.into_view()
            } else {
                view! {
                    <span class="doc-name">{label}</span>
                }.into_view()
            }}
        </div>
    }
}

#[component]
fn ProjectDocEditor<G, F>(
    label: &'static str,
    doc: G,
    on_update: F,
) -> impl IntoView
where
    G: Fn() -> Option<DocLink> + 'static,
    F: Fn(Option<DocLink>) + 'static + Clone,
{
    let initial = doc();
    let (status, set_status) = create_signal(initial.as_ref().map(|d| d.status).unwrap_or(false));
    let (url, set_url) = create_signal(initial.as_ref().and_then(|d| d.url.clone()).unwrap_or_default());

    let on_update_1 = on_update.clone();
    let on_update_2 = on_update;

    view! {
        <div class="project-doc-editor-row">
            <label class="checkbox-label">
                <input type="checkbox"
                    prop:checked=move || status.get()
                    on:change=move |ev| {
                        let new_status = event_target_checked(&ev);
                        set_status.set(new_status);
                        on_update_1(Some(DocLink {
                            name: label.to_string(),
                            url: if url.get().is_empty() { None } else { Some(url.get()) },
                            status: new_status,
                        }));
                    }
                />
                <span class="doc-label">{label}</span>
            </label>
            <input type="text" class="url-input" placeholder="URL"
                prop:value=move || url.get()
                on:input=move |ev| {
                    let new_url = event_target_value(&ev);
                    set_url.set(new_url.clone());
                    on_update_2(Some(DocLink {
                        name: label.to_string(),
                        url: if new_url.is_empty() { None } else { Some(new_url) },
                        status: status.get(),
                    }));
                }
            />
        </div>
    }
}

#[component]
fn ContractorCard(contractor: Contractor) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");
    let total = contractor.docs.len();
    let complete = contractor.docs.values().filter(|d| d.status).count();
    let is_complete = complete == total;

    // チェック状況の集計
    let checked_count = contractor.docs.values()
        .filter(|d| d.check_result.is_some())
        .count();
    let warning_count = contractor.docs.values()
        .filter(|d| d.check_result.as_ref().map(|r| r.status == "warning").unwrap_or(false))
        .count();
    let error_count = contractor.docs.values()
        .filter(|d| d.check_result.as_ref().map(|r| r.status == "error").unwrap_or(false))
        .count();

    let contractor_name = contractor.name.clone();
    let contractor_id = contractor.id.clone();

    // ドキュメントをソートして表示
    let mut docs: Vec<_> = contractor.docs.into_iter().collect();
    docs.sort_by(|a, b| a.0.cmp(&b.0));

    view! {
        <div class=format!("contractor-card {}", if is_complete { "complete" } else { "incomplete" })>
            <div class="contractor-header">
                <h4>{contractor.name}</h4>
                <span class="role">{contractor.role}</span>

                <div class="header-stats">
                    <span class="count">{complete}"/" {total}</span>

                    // チェック状況バッジ
                    {(checked_count > 0).then(|| view! {
                        <span class="checked-stats">
                            {(error_count > 0).then(|| view! {
                                <span class="stat-error" title="要対応">"!" {error_count}</span>
                            })}
                            {(warning_count > 0).then(|| view! {
                                <span class="stat-warning" title="要確認">"⚠" {warning_count}</span>
                            })}
                            <span class="stat-checked" title="チェック済み">"📋" {checked_count}</span>
                        </span>
                    })}
                </div>
            </div>

            <div class="doc-list">
                {docs.into_iter().map(|(key, status)| {
                    let label = key.replace("_", " ").chars().skip_while(|c| c.is_numeric()).collect::<String>();
                    let label = label.trim_start_matches('_').to_string();
                    let has_url = status.url.is_some();
                    let url = status.url.clone();
                    let key_click = key.clone();
                    let contractor_id_click = contractor_id.clone();

                    // チェック結果からバッジを決定
                    let check_badge = status.check_result.as_ref().map(|r| {
                        match r.status.as_str() {
                            "ok" => ("✓", "badge-ok", "チェック済み"),
                            "warning" => ("⚠", "badge-warning", "要確認"),
                            "error" => ("!", "badge-error", "要対応"),
                            _ => ("?", "badge-unknown", "不明"),
                        }
                    });

                    let last_checked = status.last_checked.clone();

                    // クリック用の変数クローン
                    let contractor_name_click = contractor_name.clone();
                    let label_click = label.clone();
                    let url_click = url.clone();
                    let key_click = key.clone();
                    let contractor_id_click = contractor_id.clone();
                    let set_view_mode = ctx.set_view_mode;

                    // ホバーツールチップ用
                    let contractor_name_hover = contractor_name.clone();
                    let label_hover = label.clone();
                    let key_hover = key.clone();
                    let check_result_hover = status.check_result.clone();
                    let last_checked_hover = status.last_checked.clone();
                    let set_context_menu = ctx.set_context_menu;
                    let context_menu = ctx.context_menu;

                    // mouseenter: 1秒後にツールチップ表示
                    let contractor_name_enter = contractor_name_hover.clone();
                    let label_enter = label_hover.clone();
                    let key_enter = key_hover.clone();
                    let check_result_enter = check_result_hover.clone();
                    let last_checked_enter = last_checked_hover.clone();
                    let on_mouse_enter = move |ev: web_sys::MouseEvent| {
                        let window = web_sys::window().unwrap();
                        // 既存タイマーをキャンセル
                        if let Some(id) = context_menu.get().hover_timer_id {
                            window.clear_timeout_with_handle(id);
                        }

                        let contractor_name = contractor_name_enter.clone();
                        let label = label_enter.clone();
                        let key = key_enter.clone();
                        let check_result = check_result_enter.clone();
                        let last_checked = last_checked_enter.clone();
                        let x = ev.client_x();
                        let y = ev.client_y();

                        let closure = Closure::once(Box::new(move || {
                            set_context_menu.set(ContextMenuState {
                                visible: true,
                                x,
                                y,
                                contractor_name,
                                doc_key: key,
                                doc_label: label,
                                check_result,
                                last_checked,
                                hover_timer_id: None,
                            });
                        }) as Box<dyn FnOnce()>);

                        let timer_id = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                            closure.as_ref().unchecked_ref(),
                            1000,
                        ).unwrap_or(0);
                        closure.forget();

                        // タイマーIDを保存（後でキャンセル用）
                        set_context_menu.update(|s| s.hover_timer_id = Some(timer_id));
                    };

                    // mouseleave: タイマーキャンセル＆ツールチップ非表示
                    let on_mouse_leave = move |_: web_sys::MouseEvent| {
                        let window = web_sys::window().unwrap();
                        let state = context_menu.get();
                        if let Some(id) = state.hover_timer_id {
                            window.clear_timeout_with_handle(id);
                        }
                        set_context_menu.set(ContextMenuState::default());
                    };

                    let on_doc_click = move |ev: web_sys::MouseEvent| {
                        ev.prevent_default();
                        if let Some(ref u) = url_click {
                            let file_type = detect_file_type(u);
                            match file_type {
                                DocFileType::Pdf | DocFileType::Image => {
                                    set_view_mode.set(ViewMode::PdfViewer {
                                        contractor: contractor_name_click.clone(),
                                        doc_type: label_click.clone(),
                                        url: u.clone(),
                                        doc_key: key_click.clone(),
                                        contractor_id: contractor_id_click.clone(),
                                    });
                                }
                                DocFileType::GoogleSpreadsheet | DocFileType::Excel => {
                                    set_view_mode.set(ViewMode::SpreadsheetViewer {
                                        contractor: contractor_name_click.clone(),
                                        doc_type: label_click.clone(),
                                        url: u.clone(),
                                        doc_key: key_click.clone(),
                                        contractor_id: contractor_id_click.clone(),
                                    });
                                }
                                _ => {
                                    // 不明な場合はURLを新規タブで開く
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.open_with_url_and_target(u, "_blank");
                                    }
                                }
                            }
                        }
                    };

                    view! {
                        <div
                            class=format!("doc-item {} {} {}",
                                if status.status { "ok" } else { "missing" },
                                if has_url { "has-link clickable" } else { "" },
                                check_badge.as_ref().map(|(_, class, _)| *class).unwrap_or("")
                            )
                            on:click=on_doc_click
                            on:mouseenter=on_mouse_enter
                            on:mouseleave=on_mouse_leave
                        >
                            // 書類状態アイコン
                            <span class="doc-icon">{if status.status { "✓" } else { "✗" }}</span>

                            // 書類名
                            <span class=format!("doc-name {}", if has_url { "doc-link" } else { "" })>
                                {label.clone()}
                            </span>

                            // チェック結果バッジ
                            {check_badge.map(|(icon, class, title)| view! {
                                <span
                                    class=format!("check-badge {}", class)
                                    title=title
                                >
                                    {icon}
                                </span>
                            })}

                            // 最終チェック日時（ホバーで表示）
                            {last_checked.map(|dt| view! {
                                <span class="last-checked" title=format!("最終チェック: {}", dt)>
                                    "📅"
                                </span>
                            })}

                            // 備考
                            {status.note.clone().map(|n| view! {
                                <span class="doc-note">{n}</span>
                            })}

                            // クリックヒント
                            {has_url.then(|| view! {
                                <span class="click-hint">"クリックで開く"</span>
                            })}
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

// ============================================
// 編集コンポーネント
// ============================================

#[component]
fn ProjectEditor(project: ProjectData) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");

    // ローカルで編集可能な状態を作成
    let (project_name, set_project_name) = create_signal(project.project_name.clone());
    let (client, set_client) = create_signal(project.client.clone());
    let (period, set_period) = create_signal(project.period.clone());
    let (project_docs, set_project_docs) = create_signal(project.project_docs.clone());
    let (contractors, set_contractors) = create_signal(project.contractors.clone());
    let (contracts, _set_contracts) = create_signal(project.contracts.clone());

    // 変更を保存
    let save_changes = move |_| {
        let updated = ProjectData {
            project_name: project_name.get(),
            client: client.get(),
            period: period.get(),
            project_docs: project_docs.get(),
            contractors: contractors.get(),
            contracts: contracts.get(),
        };
        ctx.set_project.set(Some(updated));
    };

    // 業者追加
    let add_contractor = move |_| {
        set_contractors.update(|cs| {
            let new_id = format!("contractor_{}", cs.len() + 1);
            cs.push(Contractor {
                id: new_id,
                name: "新規業者".to_string(),
                role: "".to_string(),
                docs: HashMap::new(),
            });
        });
    };

    // 業者削除
    let delete_contractor = move |idx: usize| {
        set_contractors.update(|cs| {
            if idx < cs.len() {
                cs.remove(idx);
            }
        });
    };

    // 業者更新
    let update_contractor = move |idx: usize, updated: Contractor| {
        set_contractors.update(|cs| {
            if idx < cs.len() {
                cs[idx] = updated;
            }
        });
    };

    view! {
        <div class="project-editor">
            <div class="editor-header">
                <h2>"プロジェクト編集"</h2>
                <button class="save-btn" on:click=save_changes>"変更を保存"</button>
            </div>

            <div class="editor-section">
                <h3>"基本情報"</h3>
                <div class="form-group">
                    <label>"工事名"</label>
                    <input type="text"
                        prop:value=move || project_name.get()
                        on:input=move |ev| set_project_name.set(event_target_value(&ev))
                    />
                </div>
                <div class="form-row">
                    <div class="form-group">
                        <label>"発注者"</label>
                        <input type="text"
                            prop:value=move || client.get()
                            on:input=move |ev| set_client.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="form-group">
                        <label>"工期"</label>
                        <input type="text"
                            prop:value=move || period.get()
                            on:input=move |ev| set_period.set(event_target_value(&ev))
                        />
                    </div>
                </div>
            </div>

            <div class="editor-section">
                <h3>"全体書類"</h3>
                <div class="project-docs-editor">
                    <ProjectDocEditor
                        label="施工体系図"
                        doc=move || project_docs.get().sekou_taikeizu.clone()
                        on_update=move |d| set_project_docs.update(|pd| pd.sekou_taikeizu = d)
                    />
                    <ProjectDocEditor
                        label="施工体制台帳"
                        doc=move || project_docs.get().sekou_taisei_daicho.clone()
                        on_update=move |d| set_project_docs.update(|pd| pd.sekou_taisei_daicho = d)
                    />
                    <ProjectDocEditor
                        label="下請契約書"
                        doc=move || project_docs.get().shitauke_keiyaku.clone()
                        on_update=move |d| set_project_docs.update(|pd| pd.shitauke_keiyaku = d)
                    />
                </div>
            </div>

            <div class="editor-section">
                <div class="section-header">
                    <h3>"業者一覧"</h3>
                    <button class="add-btn" on:click=add_contractor>"+ 業者追加"</button>
                </div>

                <div class="contractors-editor">
                    {move || contractors.get().into_iter().enumerate().map(|(idx, c)| {
                        let update_fn = move |updated: Contractor| update_contractor(idx, updated);
                        let delete_fn = move |_| delete_contractor(idx);
                        view! {
                            <ContractorEditor
                                contractor=c
                                on_update=update_fn
                                on_delete=delete_fn
                            />
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}

#[component]
fn ContractorEditor<F, D>(
    contractor: Contractor,
    on_update: F,
    on_delete: D,
) -> impl IntoView
where
    F: Fn(Contractor) + 'static + Clone,
    D: Fn(()) + 'static,
{
    let (name, set_name) = create_signal(contractor.name.clone());
    let (role, set_role) = create_signal(contractor.role.clone());
    let (docs, set_docs) = create_signal(contractor.docs.clone());
    let (expanded, set_expanded) = create_signal(false);

    let contractor_id = contractor.id.clone();
    let contractor_id_2 = contractor_id.clone();
    let contractor_id_3 = contractor_id.clone();

    let on_update_1 = on_update.clone();
    let on_update_2 = on_update.clone();
    let on_update_3 = on_update.clone();

    view! {
        <div class="contractor-editor">
            <div class="contractor-editor-header" on:click=move |_| set_expanded.update(|e| *e = !*e)>
                <span class="expand-icon">{move || if expanded.get() { "▼" } else { "▶" }}</span>
                <input type="text" class="name-input"
                    prop:value=move || name.get()
                    on:input={
                        let contractor_id = contractor_id.clone();
                        let on_update = on_update_1.clone();
                        move |ev| {
                            set_name.set(event_target_value(&ev));
                            on_update(Contractor {
                                id: contractor_id.clone(),
                                name: name.get(),
                                role: role.get(),
                                docs: docs.get(),
                            });
                        }
                    }
                    on:click=move |ev| ev.stop_propagation()
                />
                <input type="text" class="role-input" placeholder="役割"
                    prop:value=move || role.get()
                    on:input={
                        let contractor_id = contractor_id_2.clone();
                        let on_update = on_update_2.clone();
                        move |ev| {
                            set_role.set(event_target_value(&ev));
                            on_update(Contractor {
                                id: contractor_id.clone(),
                                name: name.get(),
                                role: role.get(),
                                docs: docs.get(),
                            });
                        }
                    }
                    on:click=move |ev| ev.stop_propagation()
                />
                <button class="delete-btn" on:click=move |ev| {
                    ev.stop_propagation();
                    on_delete(());
                }>"削除"</button>
            </div>

            {move || {
                let is_expanded = expanded.get();
                let on_update = on_update_3.clone();
                let contractor_id = contractor_id_3.clone();

                is_expanded.then(|| {
                    let mut doc_list: Vec<_> = docs.get().into_iter().collect();
                    doc_list.sort_by(|a, b| a.0.cmp(&b.0));

                    let on_update_add = on_update.clone();
                    let contractor_id_add = contractor_id.clone();

                    view! {
                        <div class="docs-editor">
                            <div class="docs-header">
                                <span>"書類一覧"</span>
                                <button class="add-btn small" on:click=move |_| {
                                    set_docs.update(|d| {
                                        for (key, _) in STANDARD_DOCS {
                                            if !d.contains_key(*key) {
                                                d.insert(key.to_string(), DocStatus {
                                                    status: false,
                                                    file: None,
                                                    url: None,
                                                    note: Some("要依頼".to_string()),
                                                    valid_from: None,
                                                    valid_until: None,
                                                    check_result: None,
                                                    last_checked: None,
                                                });
                                                break;
                                            }
                                        }
                                    });
                                    on_update_add(Contractor {
                                        id: contractor_id_add.clone(),
                                        name: name.get(),
                                        role: role.get(),
                                        docs: docs.get(),
                                    });
                                }>"+ 書類追加"</button>
                            </div>
                            {doc_list.into_iter().map(|(key, status)| {
                                let key_clone = key.clone();
                                let key_for_delete = key.clone();
                                let on_update_doc = on_update.clone();
                                let on_update_del = on_update.clone();
                                let contractor_id_doc = contractor_id.clone();
                                let contractor_id_del = contractor_id.clone();

                                let update_doc = move |updated_status: DocStatus| {
                                    set_docs.update(|d| {
                                        d.insert(key_clone.clone(), updated_status);
                                    });
                                    on_update_doc(Contractor {
                                        id: contractor_id_doc.clone(),
                                        name: name.get(),
                                        role: role.get(),
                                        docs: docs.get(),
                                    });
                                };

                                let delete_doc = move |_| {
                                    set_docs.update(|d| {
                                        d.remove(&key_for_delete);
                                    });
                                    on_update_del(Contractor {
                                        id: contractor_id_del.clone(),
                                        name: name.get(),
                                        role: role.get(),
                                        docs: docs.get(),
                                    });
                                };

                                view! {
                                    <DocEditor
                                        doc_key=key
                                        status=status
                                        on_update=update_doc
                                        on_delete=delete_doc
                                    />
                                }
                            }).collect_view()}
                        </div>
                    }
                })
            }}
        </div>
    }
}

#[component]
fn DocEditor<F, D>(
    doc_key: String,
    status: DocStatus,
    on_update: F,
    on_delete: D,
) -> impl IntoView
where
    F: Fn(DocStatus) + 'static + Clone,
    D: Fn(()) + 'static,
{
    let (doc_status, set_doc_status) = create_signal(status.status);
    let (file, set_file) = create_signal(status.file.clone().unwrap_or_default());
    let (url, set_url) = create_signal(status.url.clone().unwrap_or_default());
    let (valid_until, set_valid_until) = create_signal(status.valid_until.clone().unwrap_or_default());
    let (note, set_note) = create_signal(status.note.clone().unwrap_or_default());

    let label = doc_key.replace("_", " ").chars().skip_while(|c| c.is_numeric()).collect::<String>();
    let label = label.trim_start_matches('_').to_string();

    // 各イベント用にon_updateをクローン
    let on_update_1 = on_update.clone();
    let on_update_2 = on_update.clone();
    let on_update_3 = on_update.clone();
    let on_update_4 = on_update.clone();
    let on_update_5 = on_update;

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

    view! {
        <div class=format!("doc-editor {}", if doc_status.get() { "complete" } else { "incomplete" })>
            <div class="doc-editor-row">
                <label class="checkbox-label">
                    <input type="checkbox"
                        prop:checked=move || doc_status.get()
                        on:change=move |ev| {
                            set_doc_status.set(event_target_checked(&ev));
                            on_update_1(make_status());
                        }
                    />
                    <span class="doc-label">{label}</span>
                </label>
                <button class="delete-btn small" on:click=move |_| on_delete(())>"✕"</button>
            </div>
            <div class="doc-editor-fields">
                <input type="text" placeholder="ファイル名"
                    prop:value=move || file.get()
                    on:input=move |ev| {
                        set_file.set(event_target_value(&ev));
                        on_update_2(make_status());
                    }
                />
                <input type="text" placeholder="URL"
                    prop:value=move || url.get()
                    on:input=move |ev| {
                        set_url.set(event_target_value(&ev));
                        on_update_3(make_status());
                    }
                />
                <input type="date" placeholder="有効期限"
                    prop:value=move || valid_until.get()
                    on:input=move |ev| {
                        set_valid_until.set(event_target_value(&ev));
                        on_update_4(make_status());
                    }
                />
                <input type="text" placeholder="備考"
                    prop:value=move || note.get()
                    on:input=move |ev| {
                        set_note.set(event_target_value(&ev));
                        on_update_5(make_status());
                    }
                />
            </div>
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
// スプレッドシートビューワコンポーネント
// ============================================

#[component]
fn SpreadsheetViewer(
    contractor: String,
    doc_type: String,
    url: String,
    doc_key: String,
    contractor_id: String,
) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");

    // 未使用パラメータ
    let _ = (doc_key, contractor_id);

    let on_back = move |_| {
        ctx.set_view_mode.set(ViewMode::Dashboard);
    };

    // ローカルパス検出（H:\, C:\, /Users/ など）
    let is_local_path = url.contains(":\\") || url.starts_with("/Users/") || url.starts_with("/home/");

    // Google Sheets URLを埋め込み用に変換（堅牢なID抽出方式）
    // rtpof=true がある場合はExcelファイルなのでDrive形式でプレビュー
    let is_excel_compat = url.contains("rtpof=true");
    let embed_url = if is_local_path {
        String::new()
    } else if url.contains("docs.google.com/spreadsheets") {
        extract_spreadsheet_info(&url)
            .map(|(id, gid)| {
                if is_excel_compat {
                    // ExcelファイルはGoogle Driveのプレビューを使用
                    build_drive_preview_url(&id)
                } else {
                    build_sheets_embed_url(&id, gid.as_deref())
                }
            })
            .unwrap_or_else(|| url.clone())
    } else {
        url.clone()
    };

    let contractor_display = contractor.clone();
    let doc_type_display = doc_type.clone();
    let url_display = url.clone();

    view! {
        <div class="viewer-container spreadsheet-viewer">
            <div class="viewer-toolbar">
                <button class="back-btn" on:click=on_back>"← 戻る"</button>
                <span class="doc-info">{contractor_display}" / "{doc_type_display}</span>
                <div class="toolbar-actions">
                    // スプレッドシートのAIチェックは現在未対応
                </div>
            </div>

            <div class="viewer-content">
                {if is_local_path {
                    view! {
                        <div class="local-path-warning">
                            <p class="warning-title">"ローカルファイルはプレビューできません"</p>
                            <p class="warning-path">{url_display}</p>
                            <p class="warning-hint">"目次シートのURLをGoogle Drive Web URL形式に変更してください"</p>
                            <p class="warning-example">"例: https://docs.google.com/spreadsheets/d/ファイルID/edit"</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <iframe
                            src=embed_url
                            class="spreadsheet-frame"
                        ></iframe>
                    }.into_view()
                }}
            </div>
        </div>
    }
}

// ============================================
// PDFエディタ
// ============================================

/// Google Drive URLをiframe埋め込み用URLに変換
fn convert_to_embed_url(url: &str) -> String {
    // Google Drive file URL: https://drive.google.com/file/d/{FILE_ID}/view
    // -> Preview URL: https://drive.google.com/file/d/{FILE_ID}/preview
    if url.contains("drive.google.com/file/d/") {
        return url.replace("/view", "/preview").replace("/edit", "/preview");
    }

    // Google Docs/Sheets/Slides URL: https://docs.google.com/document/d/{FILE_ID}/edit
    // -> Preview URL: https://docs.google.com/document/d/{FILE_ID}/preview
    if url.contains("docs.google.com/") {
        return url.replace("/edit", "/preview").replace("/view", "/preview");
    }

    // その他のURLはそのまま返す
    url.to_string()
}

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
// チェック結果パネル (拡張版 T5)
// ============================================

/// 拡張版チェック結果パネルコンポーネント
#[component]
fn CheckResultPanel(
    result: CheckResultData,
    #[prop(optional)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    let status_class = match result.status.as_str() {
        "ok" => "status-ok",
        "warning" => "status-warning",
        "error" => "status-error",
        _ => "status-unknown",
    };

    let status_icon = match result.status.as_str() {
        "ok" => "✓",
        "warning" => "⚠",
        "error" => "✗",
        _ => "?",
    };

    let status_label = match result.status.as_str() {
        "ok" => "問題なし",
        "warning" => "要確認",
        "error" => "要対応",
        _ => "不明",
    };

    // 統計
    let ok_count = result.items.iter().filter(|i| i.item_type == "ok").count();
    let warning_count = result.items.iter().filter(|i| i.item_type == "warning").count();
    let error_count = result.items.iter().filter(|i| i.item_type == "error").count();

    view! {
        <div class=format!("check-result-panel {}", status_class)>
            // ヘッダー
            <div class="result-header">
                <div class="result-status-badge">
                    <span class="status-icon">{status_icon}</span>
                    <span class="status-label">{status_label}</span>
                </div>

                {on_close.map(|cb| view! {
                    <button class="close-btn" on:click=move |_| cb.call(())>"×"</button>
                })}
            </div>

            // サマリー
            <div class="result-summary">
                {result.summary.clone()}
            </div>

            // 統計バー
            <div class="result-stats">
                <span class="stat stat-ok">"OK: " {ok_count}</span>
                <span class="stat stat-warning">"警告: " {warning_count}</span>
                <span class="stat stat-error">"エラー: " {error_count}</span>
            </div>

            // チェック項目（折りたたみ可能）
            {(!result.items.is_empty()).then(|| {
                let items = result.items.clone();
                view! {
                    <details class="result-details" open>
                        <summary>"チェック項目 (" {items.len()} "件)"</summary>
                        <ul class="result-items-list">
                            {items.into_iter().map(|item| {
                                let icon = match item.item_type.as_str() {
                                    "ok" => "✓",
                                    "warning" => "⚠",
                                    "error" => "✗",
                                    "info" => "ℹ",
                                    _ => "•",
                                };
                                view! {
                                    <li class=format!("result-item item-{}", item.item_type)>
                                        <span class="item-icon">{icon}</span>
                                        <span class="item-message">{item.message}</span>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    </details>
                }
            })}

            // 未記入項目
            {(!result.missing_fields.is_empty()).then(|| {
                let fields = result.missing_fields.clone();
                view! {
                    <details class="missing-fields-details" open>
                        <summary class="missing-header">
                            "未記入項目 (" {fields.len()} "件)"
                        </summary>
                        <ul class="missing-fields-list">
                            {fields.into_iter().map(|field| view! {
                                <li class="missing-field-item">
                                    <span class="field-icon">"□"</span>
                                    <span class="field-name">{field.field}</span>
                                    <span class="field-location">"（"{field.location}"）"</span>
                                </li>
                            }).collect_view()}
                        </ul>
                    </details>
                }
            })}
        </div>
    }
}

// ============================================
// 既存チェック結果パネル
// ============================================

#[component]
fn CheckResultsPanel() -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");

    view! {
        {move || {
            let mode = ctx.check_mode.get();
            let results = ctx.check_results.get();

            (mode != CheckMode::None && !results.is_empty()).then(|| {
                let title = match mode {
                    CheckMode::Existence => "書類存在チェック結果",
                    CheckMode::Date => "日付チェック結果",
                    CheckMode::None => "",
                };

                // 結果を分類
                let errors: Vec<_> = results.iter().filter(|r| r.status == CheckStatus::Error).collect();
                let warnings: Vec<_> = results.iter().filter(|r| r.status == CheckStatus::Warning).collect();
                let oks: Vec<_> = results.iter().filter(|r| r.status == CheckStatus::Ok).collect();

                view! {
                    <div class="check-results-panel">
                        <h3>{title}</h3>

                        <div class="check-summary">
                            <span class="summary-ok">"OK: " {oks.len()}</span>
                            <span class="summary-warning">"警告: " {warnings.len()}</span>
                            <span class="summary-error">"エラー: " {errors.len()}</span>
                        </div>

                        {(!errors.is_empty()).then(|| view! {
                            <div class="check-section error-section">
                                <h4>"エラー"</h4>
                                {errors.into_iter().map(|r| view! {
                                    <div class="check-result-item error">
                                        <span class="result-contractor">{r.contractor_name.clone()}</span>
                                        <span class="result-doc">{r.doc_name.clone()}</span>
                                        <span class="result-message">{r.message.clone()}</span>
                                    </div>
                                }).collect_view()}
                            </div>
                        })}

                        {(!warnings.is_empty()).then(|| view! {
                            <div class="check-section warning-section">
                                <h4>"警告"</h4>
                                {warnings.into_iter().map(|r| view! {
                                    <div class="check-result-item warning">
                                        <span class="result-contractor">{r.contractor_name.clone()}</span>
                                        <span class="result-doc">{r.doc_name.clone()}</span>
                                        <span class="result-message">{r.message.clone()}</span>
                                    </div>
                                }).collect_view()}
                            </div>
                        })}

                        {(mode == CheckMode::Date && !oks.is_empty()).then(|| view! {
                            <div class="check-section ok-section">
                                <h4>"有効期限内"</h4>
                                {oks.into_iter().map(|r| view! {
                                    <div class="check-result-item ok">
                                        <span class="result-contractor">{r.contractor_name.clone()}</span>
                                        <span class="result-doc">{r.doc_name.clone()}</span>
                                        <span class="result-message">{r.message.clone()}</span>
                                    </div>
                                }).collect_view()}
                            </div>
                        })}
                    </div>
                }
            })
        }}
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

// ============================================
// OCRトークン可視化
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrToken {
    pub text: String,
    pub page: u32,
    pub normalized: NormalizedCoords,
    pub pixels: PixelCoords,
    pub page_size: PageSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedCoords {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelCoords {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrDocument {
    pub contractor: String,
    pub doc_type: String,
    pub image_url: String,  // PDF画像のURL
    pub tokens: Vec<OcrToken>,
}

// OCR可視化ビューのコンテキスト
#[derive(Clone)]
pub struct OcrViewContext {
    pub documents: ReadSignal<Vec<OcrDocument>>,
    pub set_documents: WriteSignal<Vec<OcrDocument>>,
    pub current_doc_index: ReadSignal<usize>,
    pub set_current_doc_index: WriteSignal<usize>,
    pub selected_token: ReadSignal<Option<usize>>,
    pub set_selected_token: WriteSignal<Option<usize>>,
    pub show_all_boxes: ReadSignal<bool>,
    pub set_show_all_boxes: WriteSignal<bool>,
}

#[component]
fn OcrViewer() -> impl IntoView {
    let ctx = use_context::<OcrViewContext>().expect("OcrViewContext not found");

    view! {
        <div class="ocr-viewer">
            <div class="ocr-header">
                <h2>"OCR座標マッピング"</h2>
                <p class="ocr-description">
                    "Document AI OCRで検出したテキストの位置を表示します。"
                    <br/>
                    "緑枠: 検出されたテキスト / 赤枠: 選択中"
                </p>
            </div>

            // ドキュメント選択
            <div class="ocr-controls">
                <select on:change=move |ev| {
                    let idx: usize = event_target_value(&ev).parse().unwrap_or(0);
                    ctx.set_current_doc_index.set(idx);
                    ctx.set_selected_token.set(None);
                }>
                    {move || ctx.documents.get().iter().enumerate().map(|(i, doc)| {
                        view! {
                            <option value=i.to_string() selected=move || ctx.current_doc_index.get() == i>
                                {format!("{} - {}", doc.contractor, doc.doc_type)}
                            </option>
                        }
                    }).collect_view()}
                </select>

                <label class="checkbox-label">
                    <input type="checkbox"
                        prop:checked=move || ctx.show_all_boxes.get()
                        on:change=move |ev| ctx.set_show_all_boxes.set(event_target_checked(&ev))
                    />
                    "全ボックス表示"
                </label>
            </div>

            // Canvas表示エリア
            <div class="ocr-canvas-container">
                <OcrCanvas />
            </div>

            // トークン一覧
            <div class="ocr-token-list">
                <h4>"検出テキスト一覧"</h4>
                <div class="token-grid">
                    {move || {
                        let docs = ctx.documents.get();
                        let idx = ctx.current_doc_index.get();
                        if idx < docs.len() {
                            docs[idx].tokens.iter().enumerate().map(|(i, token)| {
                                let is_selected = ctx.selected_token.get() == Some(i);
                                let text = token.text.clone();
                                view! {
                                    <div
                                        class=format!("token-item {}", if is_selected { "selected" } else { "" })
                                        on:click=move |_| ctx.set_selected_token.set(Some(i))
                                    >
                                        <span class="token-text">{text}</span>
                                        <span class="token-coords">
                                            {format!("({:.0}, {:.0})", token.pixels.x, token.pixels.y)}
                                        </span>
                                    </div>
                                }
                            }).collect_view()
                        } else {
                            view! { <p>"ドキュメントがありません"</p> }.into_view()
                        }
                    }}
                </div>
            </div>

            // 選択中トークンの詳細
            {move || {
                let docs = ctx.documents.get();
                let doc_idx = ctx.current_doc_index.get();
                let token_idx = ctx.selected_token.get();

                if let (Some(doc), Some(t_idx)) = (docs.get(doc_idx), token_idx) {
                    if let Some(token) = doc.tokens.get(t_idx) {
                        Some(view! {
                            <div class="token-detail">
                                <h4>"選択中: \"" {token.text.clone()} "\""</h4>
                                <table>
                                    <tr><td>"正規化座標"</td><td>{format!("x: {:.4}, y: {:.4}", token.normalized.x, token.normalized.y)}</td></tr>
                                    <tr><td>"サイズ"</td><td>{format!("w: {:.4}, h: {:.4}", token.normalized.width, token.normalized.height)}</td></tr>
                                    <tr><td>"ピクセル座標"</td><td>{format!("x: {}, y: {}", token.pixels.x, token.pixels.y)}</td></tr>
                                    <tr><td>"ピクセルサイズ"</td><td>{format!("w: {}, h: {}", token.pixels.width, token.pixels.height)}</td></tr>
                                </table>
                            </div>
                        })
                    } else { None }
                } else { None }
            }}
        </div>
    }
}

#[component]
fn OcrCanvas() -> impl IntoView {
    let ctx = use_context::<OcrViewContext>().expect("OcrViewContext not found");
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // 読み込み済み画像を保持するシグナル
    let (loaded_image, set_loaded_image) = create_signal::<Option<HtmlImageElement>>(None);
    // 現在読み込み中の画像URL
    let (loading_url, set_loading_url) = create_signal::<String>(String::new());

    // 画像読み込みエフェクト
    create_effect(move |_| {
        let docs = ctx.documents.get();
        let doc_idx = ctx.current_doc_index.get();

        if let Some(doc) = docs.get(doc_idx) {
            let image_url = doc.image_url.clone();

            // 新しい画像URLなら読み込み開始
            if !image_url.is_empty() && image_url != loading_url.get_untracked() {
                set_loading_url.set(image_url.clone());
                set_loaded_image.set(None);  // 読み込み中はクリア

                // 画像エレメントを作成
                if let Ok(img) = HtmlImageElement::new() {
                    let set_img = set_loaded_image.clone();

                    // onloadコールバック
                    let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                        // 画像読み込み完了 - 再描画トリガー
                    }) as Box<dyn FnMut(_)>);

                    img.set_onload(Some(onload.as_ref().unchecked_ref()));
                    onload.forget();  // メモリリーク注意だが、今回は問題なし

                    img.set_src(&image_url);
                    set_loaded_image.set(Some(img));
                }
            }
        }
    });

    // Canvas描画エフェクト
    create_effect(move |_| {
        let docs = ctx.documents.get();
        let doc_idx = ctx.current_doc_index.get();
        let show_all = ctx.show_all_boxes.get();
        let selected = ctx.selected_token.get();
        let img = loaded_image.get();

        if let Some(doc) = docs.get(doc_idx) {
            if let Some(canvas) = canvas_ref.get() {
                let canvas_el: &HtmlCanvasElement = &canvas;
                draw_ocr_canvas(canvas_el, doc, show_all, selected, img.as_ref());
            }
        }
    });

    view! {
        <canvas
            node_ref=canvas_ref
            class="ocr-canvas"
            width="800"
            height="1130"
        />
    }
}

fn draw_ocr_canvas(canvas: &HtmlCanvasElement, doc: &OcrDocument, show_all: bool, selected: Option<usize>, background_img: Option<&HtmlImageElement>) {
    let ctx = canvas.get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok());

    if let Some(ctx) = ctx {
        let canvas_width = canvas.width() as f64;
        let canvas_height = canvas.height() as f64;

        // 背景クリア
        ctx.set_fill_style(&JsValue::from_str("#f5f5f5"));
        ctx.fill_rect(0.0, 0.0, canvas_width, canvas_height);

        // ページサイズを取得（最初のトークンから）
        let page_size = doc.tokens.first()
            .map(|t| (t.page_size.width, t.page_size.height))
            .unwrap_or((1681.0, 2378.0));

        // スケール計算
        let scale_x = canvas_width / page_size.0;
        let scale_y = canvas_height / page_size.1;
        let scale = scale_x.min(scale_y);

        // オフセット（センタリング）
        let offset_x = (canvas_width - page_size.0 * scale) / 2.0;
        let offset_y = (canvas_height - page_size.1 * scale) / 2.0;

        // 背景画像を描画（ある場合）
        if let Some(img) = background_img {
            // 画像が読み込み完了しているか確認
            if img.complete() && img.natural_width() > 0 {
                let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(
                    img,
                    offset_x,
                    offset_y,
                    page_size.0 * scale,
                    page_size.1 * scale,
                );
            } else {
                // 画像読み込み中 - 白背景
                ctx.set_fill_style(&JsValue::from_str("#ffffff"));
                ctx.fill_rect(offset_x, offset_y, page_size.0 * scale, page_size.1 * scale);
            }
        } else {
            // 画像なし - 白背景
            ctx.set_fill_style(&JsValue::from_str("#ffffff"));
            ctx.fill_rect(offset_x, offset_y, page_size.0 * scale, page_size.1 * scale);
        }

        // ページ境界線
        ctx.set_stroke_style(&JsValue::from_str("#cccccc"));
        ctx.set_line_width(1.0);
        ctx.stroke_rect(offset_x, offset_y, page_size.0 * scale, page_size.1 * scale);

        // トークンを描画
        for (i, token) in doc.tokens.iter().enumerate() {
            let is_selected = selected == Some(i);
            let is_marker = token.text == "御" || token.text == "中" ||
                           token.text == "令" || token.text == "和" ||
                           token.text == "年" || token.text == "月" || token.text == "日" ||
                           token.text == "殿" || token.text == "様";

            // 表示するかどうか
            if !show_all && !is_selected && !is_marker {
                continue;
            }

            let x = offset_x + token.normalized.x * page_size.0 * scale;
            let y = offset_y + token.normalized.y * page_size.1 * scale;
            let w = token.normalized.width * page_size.0 * scale;
            let h = token.normalized.height * page_size.1 * scale;

            // 色設定
            let (stroke_color, fill_color, line_width) = if is_selected {
                ("#ff0000", "rgba(255, 0, 0, 0.2)", 3.0)  // 赤: 選択中
            } else if is_marker {
                ("#0066ff", "rgba(0, 102, 255, 0.15)", 2.0)  // 青: マーカー
            } else {
                ("#00aa00", "rgba(0, 170, 0, 0.1)", 1.0)  // 緑: 通常
            };

            // 塗りつぶし
            ctx.set_fill_style(&JsValue::from_str(fill_color));
            ctx.fill_rect(x, y, w, h);

            // 枠線
            ctx.set_stroke_style(&JsValue::from_str(stroke_color));
            ctx.set_line_width(line_width);
            ctx.stroke_rect(x, y, w, h);

            // テキストラベル（マーカーまたは選択中のみ）
            if is_selected || is_marker {
                ctx.set_fill_style(&JsValue::from_str(stroke_color));
                ctx.set_font("12px sans-serif");
                let _ = ctx.fill_text(&token.text, x, y - 2.0);
            }
        }

        // 凡例
        ctx.set_font("14px sans-serif");
        ctx.set_fill_style(&JsValue::from_str("#333333"));
        let _ = ctx.fill_text("凡例:", 10.0, 20.0);

        ctx.set_fill_style(&JsValue::from_str("#0066ff"));
        let _ = ctx.fill_text("■ マーカー(御/令和/年月日)", 10.0, 40.0);

        ctx.set_fill_style(&JsValue::from_str("#00aa00"));
        let _ = ctx.fill_text("■ 通常テキスト", 10.0, 60.0);

        ctx.set_fill_style(&JsValue::from_str("#ff0000"));
        let _ = ctx.fill_text("■ 選択中", 10.0, 80.0);
    }
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

    // コンテキストメニュー状態
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
                if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                    if let Ok(type_val) = js_sys::Reflect::get(&data, &JsValue::from_str("type")) {
                        if let Some(type_str) = type_val.as_string() {
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
                                    web_sys::console::log_1(&"[ai-check-result] Received".into());
                                    // AIチェック結果を受け取り、ProjectDataを更新
                                    let contractor = js_sys::Reflect::get(&data, &JsValue::from_str("contractor"))
                                        .ok().and_then(|v| v.as_string());
                                    let doc_key_raw = js_sys::Reflect::get(&data, &JsValue::from_str("docKey"))
                                        .ok().and_then(|v| v.as_string());
                                    let result_val = js_sys::Reflect::get(&data, &JsValue::from_str("result")).ok();

                                    web_sys::console::log_1(&format!("[ai-check-result] contractor={:?}, doc_key={:?}", contractor, doc_key_raw).into());

                                    if let (Some(contractor_name), Some(doc_key_raw), Some(result_js)) = (contractor, doc_key_raw, result_val) {
                                        let doc_key = doc_key_raw.trim().to_string();
                                        // 結果をCheckResultDataにデシリアライズ
                                        match serde_wasm_bindgen::from_value::<CheckResultData>(result_js.clone()) {
                                            Ok(check_result) => {
                                                web_sys::console::log_1(&format!("[ai-check-result] Deserialized: status={}", check_result.status).into());
                                                // ProjectDataを更新
                                                if let Some(mut proj) = project.get() {
                                                    let now = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();

                                                    // contractor.docsを更新
                                                    let contractor_name_trimmed = contractor_name.trim();
                                                    for contractor in &mut proj.contractors {
                                                        if contractor.name.trim() == contractor_name_trimmed {
                                                            web_sys::console::log_1(&format!("[ai-check-result] Found contractor: {}", contractor.name).into());
                                                            let doc_keys: Vec<_> = contractor.docs.keys().collect();
                                                            web_sys::console::log_1(&format!("[ai-check-result] Available docs: {:?}", doc_keys).into());
                                                            if let Some(doc_status) = contractor.docs.get_mut(&doc_key) {
                                                                doc_status.check_result = Some(check_result.clone());
                                                                doc_status.last_checked = Some(now.clone());
                                                                web_sys::console::log_1(&format!("[ai-check-result] Updated doc: {}", doc_key).into());
                                                            } else {
                                                                web_sys::console::log_1(&format!("[ai-check-result] Doc key '{}' not found", doc_key).into());
                                                            }
                                                            break;
                                                        }
                                                    }

                                                    // ローカル更新
                                                    set_project.set(Some(proj.clone()));
                                                    save_to_cache(&proj);
                                                    web_sys::console::log_1(&"[ai-check-result] Saved to cache".into());

                                                    // GASに保存
                                                    spawn_local(async move {
                                                        if let Err(e) = sync_to_gas(&proj).await {
                                                            web_sys::console::error_1(&format!("GAS保存エラー: {}", e).into());
                                                        }
                                                    });
                                                }
                                            }
                                            Err(e) => {
                                                web_sys::console::error_1(&format!("[ai-check-result] Deserialize error: {:?}", e).into());
                                            }
                                        }
                                    }
                                    set_view_mode.set(ViewMode::Dashboard);
                                }
                                "ai-check-cancel" | "back" => {
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
                                            // APIキーも自動保存
                                            auto_save_api_key_to_sheet(&url_clone).await;
                                            set_gas_syncing.set(false);
                                        });
                                    } else {
                                        // プロジェクトデータがなくてもAPIキーは保存
                                        spawn_local(async move {
                                            auto_save_api_key_to_sheet(&url_clone).await;
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

