use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FileReader, HtmlInputElement, Request, RequestInit, Response};
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
// LocalStorageキャッシュ
// ============================================

const CACHE_KEY: &str = "sekou_taisei_cache";

fn save_to_cache(project: &ProjectData) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(project) {
                let _ = storage.set_item(CACHE_KEY, &json);
            }
        }
    }
}

fn load_from_cache() -> Option<ProjectData> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let json = storage.get_item(CACHE_KEY).ok()??;
    serde_json::from_str(&json).ok()
}

fn clear_cache() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(CACHE_KEY);
        }
    }
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
    #[serde(default)]
    pub project_docs: ProjectDocs,  // 全体書類
    pub contractors: Vec<Contractor>,
    #[serde(default)]
    pub contracts: Vec<Contract>,
}

// 全体書類（施工体系図、施工体制台帳、下請契約書）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectDocs {
    #[serde(default)]
    pub sekou_taikeizu: Option<DocLink>,      // 施工体系図
    #[serde(default)]
    pub sekou_taisei_daicho: Option<DocLink>, // 施工体制台帳
    #[serde(default)]
    pub shitauke_keiyaku: Option<DocLink>,    // 下請契約書
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocLink {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub status: bool,
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
    #[serde(default)]
    pub valid_from: Option<String>,  // 有効期間開始 (YYYY-MM-DD)
    #[serde(default)]
    pub valid_until: Option<String>, // 有効期限 (YYYY-MM-DD)
}

// チェック結果
#[derive(Debug, Clone, PartialEq)]
pub enum CheckMode {
    None,
    Existence,  // 書類存在チェック
    Date,       // 日付チェック
}

// ============================================
// ビューモード
// ============================================

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Dashboard,
    OcrViewer,
    PdfViewer(String),
    SpreadsheetViewer(String), // contractor_name_doc_type
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Dashboard
    }
}

// ============================================
// スプレッドシートビューア
// ============================================

#[derive(Clone, PartialEq)]
pub enum SpreadsheetType {
    GoogleSpreadsheet,
    Excel,
    Unknown,
}

#[derive(Clone)]
pub struct SpreadsheetViewerContext {
    pub doc_url: RwSignal<String>,
    pub doc_type: RwSignal<SpreadsheetType>,
    pub gemini_check_result: RwSignal<Option<GeminiCheckResult>>,
    pub is_checking: RwSignal<bool>,
}

#[derive(Clone, Debug)]
pub struct GeminiCheckResult {
    pub status: GeminiCheckStatus,
    pub messages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeminiCheckStatus {
    Ok,
    Warning,
    Error,
}

fn detect_spreadsheet_type(url: &str) -> SpreadsheetType {
    if url.contains("docs.google.com/spreadsheets") {
        SpreadsheetType::GoogleSpreadsheet
    } else if url.ends_with(".xlsx") || url.ends_with(".xls") {
        SpreadsheetType::Excel
    } else {
        SpreadsheetType::Unknown
    }
}

fn open_in_browser(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(url, "_blank");
    }
}

// ダミーのGEMINI確認関数（Task Dで実装）
async fn check_with_gemini(_spreadsheet_id: &str, check_type: &str) -> GeminiCheckResult {
    // TODO: Task Dで実際のAPI呼び出しを実装
    // 今はダミーデータを返す
    gloo::timers::future::TimeoutFuture::new(1000).await;

    match check_type {
        "作業員名簿" => GeminiCheckResult {
            status: GeminiCheckStatus::Warning,
            messages: vec![
                "✓ 作業員名簿の必須項目がすべて入力されています".to_string(),
                "⚠ 資格欄に記載漏れの可能性があります".to_string(),
            ],
        },
        "暴対法誓約書" => GeminiCheckResult {
            status: GeminiCheckStatus::Ok,
            messages: vec![
                "✓ 誓約書の形式は適切です".to_string(),
                "✓ 必要な署名が確認できます".to_string(),
            ],
        },
        _ => GeminiCheckResult {
            status: GeminiCheckStatus::Ok,
            messages: vec![
                "✓ 書類の内容を確認しました".to_string(),
            ],
        },
    }
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
}

// スプレッドシートビューア用データ
#[derive(Clone, Default)]
pub struct SpreadsheetViewerData {
    pub doc_name: String,
    pub doc_url: String,
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
// チェック結果パネル
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
// スプレッドシートビューアコンポーネント
// ============================================

#[component]
fn SpreadsheetViewer(
    doc_name: String,
    doc_url: String,
    on_back: Callback<()>,
) -> impl IntoView {
    let doc_type = detect_spreadsheet_type(&doc_url);
    let (is_checking, set_is_checking) = create_signal(false);
    let (check_result, set_check_result) = create_signal(None::<GeminiCheckResult>);

    let url_for_open = doc_url.clone();
    let doc_name_for_check = doc_name.clone();
    let url_for_check = doc_url.clone();

    let on_open_click = move |_| {
        open_in_browser(&url_for_open);
    };

    let on_check_click = move |_| {
        let doc_name = doc_name_for_check.clone();
        let url = url_for_check.clone();
        set_is_checking.set(true);
        spawn_local(async move {
            // スプレッドシートIDを抽出（Google Spreadsheetsの場合）
            let spreadsheet_id = if url.contains("docs.google.com/spreadsheets") {
                url.split("/d/")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or(&url)
            } else {
                &url
            };
            let result = check_with_gemini(spreadsheet_id, &doc_name).await;
            set_check_result.set(Some(result));
            set_is_checking.set(false);
        });
    };

    let type_icon = match doc_type {
        SpreadsheetType::GoogleSpreadsheet => "📊",
        SpreadsheetType::Excel => "📗",
        SpreadsheetType::Unknown => "📄",
    };

    let type_message = match doc_type {
        SpreadsheetType::GoogleSpreadsheet => "このファイルはGoogle スプレッドシートです",
        SpreadsheetType::Excel => "このファイルはExcelファイルです",
        SpreadsheetType::Unknown => "このファイルはスプレッドシートです",
    };

    view! {
        <div class="spreadsheet-viewer">
            <div class="spreadsheet-header">
                <button class="back-btn" on:click=move |_| on_back.call(())>"← 戻る"</button>
                <span class="spreadsheet-doc-name">"書類名: " {doc_name.clone()}</span>
                <button class="close-btn" on:click=move |_| on_back.call(())>"✕"</button>
            </div>

            <div class="spreadsheet-content">
                <span class="spreadsheet-icon">{type_icon}</span>
                <p class="spreadsheet-message">{type_message}</p>

                <button class="open-button" on:click=on_open_click>
                    "ブラウザで開く"
                </button>

                <div class="gemini-section">
                    <h4>"--- GEMINI確認 ---"</h4>
                    <button
                        class="check-button"
                        on:click=on_check_click
                        disabled=move || is_checking.get()
                    >
                        {move || if is_checking.get() { "確認中..." } else { "内容をチェック" }}
                    </button>

                    {move || check_result.get().map(|result| {
                        let status_class = match result.status {
                            GeminiCheckStatus::Ok => "ok",
                            GeminiCheckStatus::Warning => "warning",
                            GeminiCheckStatus::Error => "error",
                        };
                        view! {
                            <div class="check-result-container">
                                <p class="check-result-label">"確認結果:"</p>
                                <div class=format!("check-result {}", status_class)>
                                    {result.messages.into_iter().map(|msg| view! {
                                        <p class="check-result-message">{msg}</p>
                                    }).collect_view()}
                                </div>
                            </div>
                        }
                    })}
                </div>
            </div>
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

// JSONダウンロード用関数
fn download_json(project: &ProjectData) {
    if let Ok(json) = serde_json::to_string_pretty(project) {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                // Blobを作成
                let blob_parts = js_sys::Array::new();
                blob_parts.push(&JsValue::from_str(&json));
                let options = web_sys::BlobPropertyBag::new();
                options.set_type("application/json");

                if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                        if let Ok(a) = document.create_element("a") {
                            let _ = a.set_attribute("href", &url);
                            let filename = format!("{}.json", project.project_name.replace(" ", "_"));
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

    // プロジェクトデータのグローバル状態
    let (project, set_project) = create_signal(None::<ProjectData>);
    let (loading, set_loading) = create_signal(false);
    let (error_msg, set_error_msg) = create_signal(None::<String>);
    let (check_mode, set_check_mode) = create_signal(CheckMode::None);
    let (check_results, set_check_results) = create_signal(Vec::<CheckResult>::new());
    let (edit_mode, set_edit_mode) = create_signal(false);
    let (view_mode, set_view_mode) = create_signal(ViewMode::Dashboard);
    let (spreadsheet_data, _set_spreadsheet_data) = create_signal(SpreadsheetViewerData::default());

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
    };
    provide_context(ctx.clone());

    // 初期読み込み: URLハッシュ → キャッシュ の順で試行
    create_effect(move |_| {
        if project.get().is_none() {
            if let Some(data) = get_hash_data() {
                set_project.set(Some(data.clone()));
                save_to_cache(&data);
            } else if let Some(data) = load_from_cache() {
                set_project.set(Some(data));
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
        set_menu_open.set(false);
        if let Some(p) = project.get() {
            if let Some(url) = set_hash_data(&p) {
                if let Some(window) = web_sys::window() {
                    let clipboard = window.navigator().clipboard();
                    let _ = clipboard.write_text(&url);
                    set_copy_success.set(true);
                    spawn_local(async move {
                        gloo::timers::future::TimeoutFuture::new(2000).await;
                        set_copy_success.set(false);
                    });
                }
            }
        }
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

    // チェック結果クリア
    let on_clear_check = move |_| {
        set_check_mode.set(CheckMode::None);
        set_check_results.set(Vec::new());
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
                <h1>"施工体制チェッカー"</h1>

                // 編集モード表示
                {move || edit_mode.get().then(|| view! {
                    <span class="edit-mode-badge">"編集中"</span>
                })}

                // チェックモード表示
                {move || {
                    let mode = check_mode.get();
                    (mode != CheckMode::None).then(|| {
                        let label = match mode {
                            CheckMode::Existence => "書類存在チェック中",
                            CheckMode::Date => "日付チェック中",
                            CheckMode::None => "",
                        };
                        view! {
                            <span class="check-mode-badge" on:click=on_clear_check>
                                {label} " ✕"
                            </span>
                        }
                    })
                }}

                // 三点メニュー
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
                            <hr class="menu-divider" />
                            <button class="menu-item" on:click=on_export_json disabled=move || project.get().is_none()>
                                "JSONエクスポート"
                            </button>
                            <button class="menu-item" on:click=generate_share_url disabled=move || project.get().is_none()>
                                {move || if copy_success.get() { "URLをコピーしました!" } else { "共有URLを生成" }}
                            </button>
                            <hr class="menu-divider" />
                            <a class="menu-item" href="https://github.com/YuujiKamura/SekouTaiseiMaker" target="_blank" rel="noopener">
                                "GitHub リポジトリ ↗"
                            </a>
                            <a class="menu-item" href="https://github.com/YuujiKamura/SekouTaiseiMaker/actions" target="_blank" rel="noopener">
                                "GitHub Actions ↗"
                            </a>
                            <hr class="menu-divider" />
                            <button class="menu-item danger" on:click=on_clear_cache>
                                "キャッシュクリア"
                            </button>
                        </div>
                    })}
                </div>
            </header>

            <main class="container">
                {move || {
                    match view_mode.get() {
                        ViewMode::Dashboard => view! {
                            <>
                                <Dashboard />
                                <CheckResultsPanel />
                            </>
                        }.into_view(),
                        ViewMode::SpreadsheetViewer(ref _id) => {
                            let data = spreadsheet_data.get();
                            let on_back = Callback::new(move |_| {
                                set_view_mode.set(ViewMode::Dashboard);
                            });
                            view! {
                                <SpreadsheetViewer
                                    doc_name=data.doc_name.clone()
                                    doc_url=data.doc_url.clone()
                                    on_back=on_back
                                />
                            }.into_view()
                        },
                        _ => view! {
                            <Dashboard />
                        }.into_view(),
                    }
                }}
            </main>
        </div>
    }
}


fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
