use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FileReader, HtmlInputElement, Request, RequestInit, RequestMode, Response};
use std::collections::HashMap;

// Base64エンコード/デコード（web_sys経由）
fn encode_base64(data: &str) -> Option<String> {
    let window = web_sys::window()?;
    window.btoa(data).ok()
}

fn decode_base64(data: &str) -> Option<String> {
    let window = web_sys::window()?;
    window.atob(data).ok()
}

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

// ============================================
// 施工体制ダッシュボード用データ構造
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectData {
    pub project_name: String,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub period: String,
    pub contractors: Vec<Contractor>,
    #[serde(default)]
    pub contracts: Vec<Contract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contractor {
    pub id: String,
    pub name: String,
    pub role: String,
    pub docs: HashMap<String, DocStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocStatus {
    pub status: bool,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
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
// 既存のスプレッドシート解析用データ構造
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub row: usize,
    pub col: usize,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub fields: Vec<SchemaField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedField {
    pub id: String,
    pub label: String,
    pub value: String,
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedResult {
    pub sheet_name: String,
    pub schema_name: String,
    pub fields: Vec<ExtractedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpreadsheetData {
    pub spreadsheet_name: String,
    pub spreadsheet_id: String,
    pub sheets: HashMap<String, SheetData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SheetData {
    pub rows: usize,
    pub data: Vec<Vec<String>>,
}

// ============================================
// ダッシュボードコンポーネント
// ============================================

#[component]
fn Dashboard() -> impl IntoView {
    let (project, set_project) = create_signal(None::<ProjectData>);
    let (loading, set_loading) = create_signal(false);
    let (error_msg, set_error_msg) = create_signal(None::<String>);
    let (share_url, set_share_url) = create_signal(None::<String>);
    let (copy_success, set_copy_success) = create_signal(false);

    // ページ読み込み時にURLハッシュからデータを取得
    create_effect(move |_| {
        if project.get().is_none() {
            if let Some(data) = get_hash_data() {
                set_project.set(Some(data));
            }
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
    };

    // サンプルデータ読み込み
    let load_sample = move |_| {
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
            if let Some(url) = set_hash_data(&p) {
                set_share_url.set(Some(url.clone()));
                // クリップボードにコピー
                if let Some(window) = web_sys::window() {
                    let clipboard = window.navigator().clipboard();
                    let _ = clipboard.write_text(&url);
                    set_copy_success.set(true);
                    // 2秒後にリセット
                    spawn_local(async move {
                        gloo::timers::future::TimeoutFuture::new(2000).await;
                        set_copy_success.set(false);
                    });
                }
            }
        }
    };

    view! {
        <div class="dashboard">
            <h2>"施工体制ダッシュボード"</h2>

            <div class="load-section">
                <div class="upload-area">
                    <h3>"プロジェクトJSON"</h3>
                    <input type="file" accept=".json" on:change=on_file_change />
                </div>
                <button on:click=load_sample disabled=move || loading.get()>
                    {move || if loading.get() { "読込中..." } else { "サンプル読込" }}
                </button>
            </div>

            {move || error_msg.get().map(|e| view! {
                <p class="status error">{e}</p>
            })}

            {move || project.get().map(|p| view! {
                <div class="share-section">
                    <button on:click=generate_share_url class="share-btn">
                        {move || if copy_success.get() { "コピーしました!" } else { "共有URLを生成" }}
                    </button>
                    {move || share_url.get().map(|url| view! {
                        <input type="text" class="share-url" readonly value=url />
                    })}
                </div>
                <ProjectView project=p />
            })}
        </div>
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

            <div class="contractors-grid">
                {project.contractors.into_iter().map(|c| view! {
                    <ContractorCard contractor=c />
                }).collect_view()}
            </div>

            {(!project.contracts.is_empty()).then(|| view! {
                <div class="contracts-section">
                    <h4>"契約書類"</h4>
                    <div class="contracts-list">
                        {project.contracts.into_iter().map(|c| view! {
                            <div class="contract-item">
                                {if let Some(url) = c.url {
                                    view! {
                                        <a class="contract-link" href=url target="_blank" rel="noopener">
                                            {c.name.clone()}
                                        </a>
                                    }.into_view()
                                } else {
                                    view! {
                                        <span class="contract-name">{c.name.clone()}</span>
                                    }.into_view()
                                }}
                                {c.contractor.map(|ct| view! {
                                    <span class="contract-contractor">{ct}</span>
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
fn ContractorCard(contractor: Contractor) -> impl IntoView {
    let total = contractor.docs.len();
    let complete = contractor.docs.values().filter(|d| d.status).count();
    let is_complete = complete == total;

    // ドキュメントをソートして表示
    let mut docs: Vec<_> = contractor.docs.into_iter().collect();
    docs.sort_by(|a, b| a.0.cmp(&b.0));

    view! {
        <div class=format!("contractor-card {}", if is_complete { "complete" } else { "incomplete" })>
            <div class="contractor-header">
                <h4>{contractor.name}</h4>
                <span class="role">{contractor.role}</span>
                <span class="count">{complete}"/" {total}</span>
            </div>

            <div class="doc-list">
                {docs.into_iter().map(|(key, status)| {
                    let label = key.replace("_", " ").chars().skip_while(|c| c.is_numeric()).collect::<String>();
                    let label = label.trim_start_matches('_').to_string();
                    let has_url = status.url.is_some();
                    let url = status.url.clone();
                    view! {
                        <div class=format!("doc-item {} {}",
                            if status.status { "ok" } else { "missing" },
                            if has_url { "has-link" } else { "" }
                        )>
                            <span class="doc-icon">{if status.status { "✓" } else { "✗" }}</span>
                            {if let Some(u) = url {
                                view! {
                                    <a class="doc-name doc-link" href=u target="_blank" rel="noopener">{label}</a>
                                }.into_view()
                            } else {
                                view! {
                                    <span class="doc-name">{label}</span>
                                }.into_view()
                            }}
                            {status.note.map(|n| view! {
                                <span class="doc-note">{n}</span>
                            })}
                        </div>
                    }
                }).collect_view()}
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
// メインアプリ（タブ切り替え）
// ============================================

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Dashboard,
    Parser,
}

#[component]
fn App() -> impl IntoView {
    let (current_tab, set_current_tab) = create_signal(Tab::Dashboard);

    view! {
        <div class="app">
            <header class="app-header">
                <h1>"施工体制メーカー"</h1>
                <nav class="tabs">
                    <button
                        class=move || if current_tab.get() == Tab::Dashboard { "active" } else { "" }
                        on:click=move |_| set_current_tab.set(Tab::Dashboard)
                    >
                        "ダッシュボード"
                    </button>
                    <button
                        class=move || if current_tab.get() == Tab::Parser { "active" } else { "" }
                        on:click=move |_| set_current_tab.set(Tab::Parser)
                    >
                        "シート解析"
                    </button>
                </nav>
            </header>

            <main class="container">
                {move || match current_tab.get() {
                    Tab::Dashboard => view! { <Dashboard /> }.into_view(),
                    Tab::Parser => view! { <ParserView /> }.into_view(),
                }}
            </main>
        </div>
    }
}

// ============================================
// 既存のシート解析機能
// ============================================

const GAS_URL: &str = "https://script.google.com/macros/s/AKfycby0WMayuYSyQM7msOVvMypjq3Tne10bLaPeiuVSre2YWJyu7wxgEkyeIKgQH2Zt_zsUBw/exec";

fn extract_fields(schema: &Schema, sheet_name: &str, data: &[Vec<String>]) -> ParsedResult {
    let mut fields = Vec::new();
    for field_def in &schema.fields {
        let value = data
            .get(field_def.row)
            .and_then(|row| row.get(field_def.col))
            .cloned()
            .unwrap_or_default();
        fields.push(ExtractedField {
            id: field_def.id.clone(),
            label: field_def.label.clone(),
            value,
            row: field_def.row,
            col: field_def.col,
        });
    }
    ParsedResult {
        sheet_name: sheet_name.to_string(),
        schema_name: schema.name.clone(),
        fields,
    }
}

fn auto_detect_fields(sheet_name: &str, data: &[Vec<String>]) -> ParsedResult {
    let mut fields = Vec::new();
    let patterns = [
        ("会社名", "company_name"), ("工事名称", "project_name"),
        ("工事名", "project_name"), ("発注者", "client"),
        ("工期", "period"), ("請負代金", "contract_amount"),
    ];
    for (row_idx, row) in data.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            for (pattern, field_id) in &patterns {
                if cell.contains(pattern) {
                    let value = row.get(col_idx + 1)
                        .filter(|v| !v.is_empty())
                        .or_else(|| row.get(col_idx + 2).filter(|v| !v.is_empty()))
                        .cloned()
                        .unwrap_or_default();
                    if !value.is_empty() && !value.contains(pattern) {
                        fields.push(ExtractedField {
                            id: field_id.to_string(),
                            label: pattern.to_string(),
                            value,
                            row: row_idx,
                            col: col_idx,
                        });
                    }
                }
            }
        }
    }
    ParsedResult { sheet_name: sheet_name.to_string(), schema_name: "自動検出".to_string(), fields }
}

fn extract_sheet_id(input: &str) -> String {
    if input.contains("/d/") {
        input.split("/d/").nth(1)
            .map(|s| s.split('/').next().unwrap_or(s))
            .unwrap_or(input).to_string()
    } else {
        input.to_string()
    }
}

async fn fetch_spreadsheet(sheet_id: &str) -> Result<SpreadsheetData, String> {
    if sheet_id.is_empty() {
        return Err("スプレッドシートIDを入力してください".to_string());
    }
    let url = format!("{}?id={}", GAS_URL, sheet_id);
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Request作成失敗: {:?}", e))?;
    let window = web_sys::window().ok_or("windowがありません")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await.map_err(|e| format!("fetch失敗: {:?}", e))?;
    let resp: Response = resp_value.dyn_into().map_err(|_| "Responseへの変換失敗")?;
    let json = JsFuture::from(resp.json().map_err(|e| format!("json()失敗: {:?}", e))?)
        .await.map_err(|e| format!("JSON解析失敗: {:?}", e))?;
    serde_wasm_bindgen::from_value(json).map_err(|e| format!("デシリアライズ失敗: {:?}", e))
}

#[component]
fn ParserView() -> impl IntoView {
    let (schema, set_schema) = create_signal(None::<Schema>);
    let (spreadsheet, set_spreadsheet) = create_signal(None::<SpreadsheetData>);
    let (parsed_result, set_parsed_result) = create_signal(None::<ParsedResult>);
    let (use_auto_detect, set_use_auto_detect) = create_signal(true);
    let (sheet_url_input, set_sheet_url_input) = create_signal(String::new());
    let (loading, set_loading) = create_signal(false);
    let (error_msg, set_error_msg) = create_signal(None::<String>);

    let on_schema_change = move |ev: web_sys::Event| {
        let input: HtmlInputElement = event_target(&ev);
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();
                let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Ok(result) = reader_clone.result() {
                        if let Some(text) = result.as_string() {
                            if let Ok(s) = serde_json::from_str::<Schema>(&text) {
                                set_schema.set(Some(s));
                                set_use_auto_detect.set(false);
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                let _ = reader.read_as_text(&file);
            }
        }
    };

    let on_data_change = move |ev: web_sys::Event| {
        let input: HtmlInputElement = event_target(&ev);
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();
                let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Ok(result) = reader_clone.result() {
                        if let Some(text) = result.as_string() {
                            if let Ok(data) = serde_json::from_str::<SpreadsheetData>(&text) {
                                set_spreadsheet.set(Some(data));
                                set_parsed_result.set(None);
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                let _ = reader.read_as_text(&file);
            }
        }
    };

    let on_sheet_select = move |sheet_name: String| {
        if let Some(data) = spreadsheet.get() {
            if let Some(sheet) = data.sheets.get(&sheet_name) {
                let result = if use_auto_detect.get() {
                    auto_detect_fields(&sheet_name, &sheet.data)
                } else if let Some(s) = schema.get() {
                    extract_fields(&s, &sheet_name, &sheet.data)
                } else {
                    auto_detect_fields(&sheet_name, &sheet.data)
                };
                set_parsed_result.set(Some(result));
            }
        }
    };

    let on_sheet_fetch = move |_| {
        let url_input = sheet_url_input.get();
        let sheet_id = extract_sheet_id(&url_input);
        spawn_local(async move {
            set_loading.set(true);
            set_error_msg.set(None);
            match fetch_spreadsheet(&sheet_id).await {
                Ok(data) => { set_spreadsheet.set(Some(data)); set_parsed_result.set(None); }
                Err(e) => { set_error_msg.set(Some(e)); }
            }
            set_loading.set(false);
        });
    };

    view! {
        <div class="parser-view">
            <h2>"シート解析"</h2>

            <div class="gas-section">
                <h3>"Google スプレッドシート"</h3>
                <div class="input-group">
                    <input type="text" placeholder="URL または ID"
                        prop:value=move || sheet_url_input.get()
                        on:input=move |ev| set_sheet_url_input.set(event_target_value(&ev)) />
                    <button on:click=on_sheet_fetch disabled=move || loading.get()>
                        {move || if loading.get() { "読込中..." } else { "読み込む" }}
                    </button>
                </div>
                {move || error_msg.get().map(|e| view! { <p class="status error">{e}</p> })}
            </div>

            <div class="upload-section">
                <div class="upload-area">
                    <h3>"スキーマ (任意)"</h3>
                    <input type="file" accept=".json" on:change=on_schema_change />
                    {move || schema.get().map(|s| view! { <p class="status success">"スキーマ: " {s.name}</p> })}
                </div>
                <div class="upload-area">
                    <h3>"ローカルJSON"</h3>
                    <input type="file" accept=".json" on:change=on_data_change />
                </div>
            </div>

            <div class="mode-toggle">
                <label>
                    <input type="checkbox" checked=move || use_auto_detect.get()
                        on:change=move |ev| set_use_auto_detect.set(event_target_checked(&ev)) />
                    " 自動検出モード"
                </label>
            </div>

            {move || spreadsheet.get().map(|data| {
                let sheets: Vec<String> = data.sheets.keys().cloned().collect();
                view! {
                    <div class="sheet-preview">
                        <h3>{data.spreadsheet_name.clone()}</h3>
                        <div class="sheet-buttons">
                            {sheets.into_iter().map(|name| {
                                let n = name.clone();
                                view! { <button on:click=move |_| on_sheet_select(n.clone())>{name}</button> }
                            }).collect_view()}
                        </div>
                    </div>
                }
            })}

            {move || parsed_result.get().map(|result| view! {
                <div class="sheet-preview">
                    <h3>"解析結果: " {result.sheet_name.clone()}</h3>
                    <div class="field-list">
                        {result.fields.iter().filter(|f| !f.value.is_empty()).map(|f| view! {
                            <div class="field-card">
                                <h4>{f.label.clone()}</h4>
                                <div class="value">{f.value.clone()}</div>
                            </div>
                        }).collect_view()}
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
