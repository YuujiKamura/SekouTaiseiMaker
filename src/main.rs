use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FileReader, HtmlInputElement, Request, RequestInit, RequestMode, Response};
use std::collections::HashMap;

// GAS WebアプリURL
const GAS_URL: &str = "https://script.google.com/macros/s/AKfycbwEWi2NQ9MmQKRoaWkgZA5mgSIiCrzg4KDcM1_X6NB53stmX0Kv0Kz3soIEDq6qkkcaUQ/exec";

/// スキーマのフィールド定義
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

/// スキーマ定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub fields: Vec<SchemaField>,
}

/// 抽出されたフィールド値
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedField {
    pub id: String,
    pub label: String,
    pub value: String,
    pub row: usize,
    pub col: usize,
}

/// 解析結果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedResult {
    pub sheet_name: String,
    pub schema_name: String,
    pub fields: Vec<ExtractedField>,
}

/// スプレッドシートデータ
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

/// フォルダ内スプレッドシート一覧
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FolderContents {
    pub folder_id: String,
    pub folder_name: String,
    pub spreadsheets: Vec<SpreadsheetListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadsheetListItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
}

/// スキーマを使ってシートからフィールドを抽出
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

/// 自動検出モード（スキーマなし）
fn auto_detect_fields(sheet_name: &str, data: &[Vec<String>]) -> ParsedResult {
    let mut fields = Vec::new();

    let patterns = [
        ("会社名", "company_name"),
        ("事業者ID", "company_id"),
        ("事業所名", "office_name"),
        ("許可業種", "permit_type"),
        ("許可番号", "permit_number"),
        ("工事名称", "project_name"),
        ("工事内容", "project_content"),
        ("発注者", "client"),
        ("工期", "period"),
        ("元請", "prime_contractor"),
        ("下請", "subcontractor"),
        ("工事名", "project_name"),
        ("担当工事内容", "work_content"),
        ("工事場所", "location"),
        ("請負代金", "contract_amount"),
    ];

    for (row_idx, row) in data.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            for (pattern, field_id) in &patterns {
                if cell.contains(pattern) {
                    // 値は同じ行の後ろか次の行にある
                    let value = row.get(col_idx + 1)
                        .filter(|v| !v.is_empty())
                        .or_else(|| row.get(col_idx + 2).filter(|v| !v.is_empty()))
                        .or_else(|| row.get(col_idx + 3).filter(|v| !v.is_empty()))
                        .or_else(|| row.get(col_idx + 4).filter(|v| !v.is_empty()))
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

            // 金額検出
            if cell.starts_with('¥') || cell.starts_with('\\') {
                let clean = cell.replace(['¥', '\\', ',', ' '], "");
                if clean.parse::<i64>().is_ok() {
                    fields.push(ExtractedField {
                        id: format!("amount_{}_{}", row_idx, col_idx),
                        label: "金額".to_string(),
                        value: cell.clone(),
                        row: row_idx,
                        col: col_idx,
                    });
                }
            }

            // 日付検出（令和）
            if cell.contains("令和") && cell.contains("年") {
                fields.push(ExtractedField {
                    id: format!("date_{}_{}", row_idx, col_idx),
                    label: "日付".to_string(),
                    value: cell.clone(),
                    row: row_idx,
                    col: col_idx,
                });
            }
        }
    }

    ParsedResult {
        sheet_name: sheet_name.to_string(),
        schema_name: "自動検出".to_string(),
        fields,
    }
}

/// URLからフォルダIDを抽出
fn extract_folder_id(input: &str) -> String {
    // https://drive.google.com/drive/folders/FOLDER_ID 形式
    if input.contains("/folders/") {
        input
            .split("/folders/")
            .nth(1)
            .map(|s| s.split(['?', '#']).next().unwrap_or(s))
            .unwrap_or(input)
            .to_string()
    } else {
        // IDそのまま
        input.to_string()
    }
}

/// GASからフォルダ内のスプレッドシート一覧を取得
async fn fetch_folder_contents(folder_input: &str) -> Result<FolderContents, String> {
    if folder_input.is_empty() {
        return Err("フォルダURLまたはIDを入力してください".to_string());
    }
    let folder_id = extract_folder_id(folder_input);
    let url = format!("{}?folder={}", GAS_URL, folder_id);

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)
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

    let data: FolderContents = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("デシリアライズ失敗: {:?}", e))?;

    Ok(data)
}

/// GASからスプレッドシートを取得
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
        .await
        .map_err(|e| format!("fetch失敗: {:?}", e))?;

    let resp: Response = resp_value.dyn_into()
        .map_err(|_| "Responseへの変換失敗")?;

    let json = JsFuture::from(resp.json().map_err(|e| format!("json()失敗: {:?}", e))?)
        .await
        .map_err(|e| format!("JSON解析失敗: {:?}", e))?;

    let data: SpreadsheetData = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("デシリアライズ失敗: {:?}", e))?;

    Ok(data)
}

#[component]
fn App() -> impl IntoView {
    let (schema, set_schema) = create_signal(None::<Schema>);
    let (spreadsheet, set_spreadsheet) = create_signal(None::<SpreadsheetData>);
    let (parsed_result, set_parsed_result) = create_signal(None::<ParsedResult>);
    let (use_auto_detect, set_use_auto_detect) = create_signal(true);
    let (folder_id_input, set_folder_id_input) = create_signal(String::new());
    let (folder_contents, set_folder_contents) = create_signal(None::<FolderContents>);
    let (loading, set_loading) = create_signal(false);
    let (error_msg, set_error_msg) = create_signal(None::<String>);

    // スキーマファイル読み込み
    let on_schema_change = move |ev: web_sys::Event| {
        let input: HtmlInputElement = event_target(&ev);
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();

                let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Ok(result) = reader_clone.result() {
                        if let Some(text) = result.as_string() {
                            match serde_json::from_str::<Schema>(&text) {
                                Ok(s) => {
                                    set_schema.set(Some(s));
                                    set_use_auto_detect.set(false);
                                }
                                Err(e) => {
                                    web_sys::console::log_1(&format!("Schema parse error: {}", e).into());
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

    // データファイル読み込み
    let on_data_change = move |ev: web_sys::Event| {
        let input: HtmlInputElement = event_target(&ev);
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();

                let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Ok(result) = reader_clone.result() {
                        if let Some(text) = result.as_string() {
                            match serde_json::from_str::<SpreadsheetData>(&text) {
                                Ok(data) => {
                                    set_spreadsheet.set(Some(data));
                                    set_parsed_result.set(None);
                                }
                                Err(e) => {
                                    web_sys::console::log_1(&format!("Data parse error: {}", e).into());
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

    // シート選択時の解析
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

    // フォルダ一覧取得
    let on_folder_fetch = move |_| {
        let folder_id = folder_id_input.get();
        spawn_local(async move {
            set_loading.set(true);
            set_error_msg.set(None);

            match fetch_folder_contents(&folder_id).await {
                Ok(contents) => {
                    set_folder_contents.set(Some(contents));
                    set_spreadsheet.set(None);
                    set_parsed_result.set(None);
                }
                Err(e) => {
                    set_error_msg.set(Some(e));
                }
            }

            set_loading.set(false);
        });
    };

    // スプレッドシート選択
    let on_spreadsheet_select = move |sheet_id: String| {
        spawn_local(async move {
            set_loading.set(true);
            set_error_msg.set(None);

            match fetch_spreadsheet(&sheet_id).await {
                Ok(data) => {
                    set_spreadsheet.set(Some(data));
                    set_parsed_result.set(None);
                }
                Err(e) => {
                    set_error_msg.set(Some(e));
                }
            }

            set_loading.set(false);
        });
    };

    view! {
        <div class="container">
            <h1>"施工体制メーカー"</h1>

            <div class="gas-section">
                <h3>"Google Drive フォルダから読み込み"</h3>
                <div class="input-group">
                    <input
                        type="text"
                        placeholder="フォルダURL または ID"
                        prop:value=move || folder_id_input.get()
                        on:input=move |ev| set_folder_id_input.set(event_target_value(&ev))
                    />
                    <button
                        on:click=on_folder_fetch
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() { "読み込み中..." } else { "フォルダを開く" }}
                    </button>
                </div>
                {move || error_msg.get().map(|e| view! {
                    <p class="status error">{e}</p>
                })}
            </div>

            {move || folder_contents.get().map(|contents| {
                let folder_name = contents.folder_name.clone();
                view! {
                    <div class="folder-list">
                        <h3>"フォルダ: " {folder_name}</h3>
                        <div class="spreadsheet-list">
                            {contents.spreadsheets.into_iter().map(|ss| {
                                let id = ss.id.clone();
                                let name = ss.name.clone();
                                view! {
                                    <button
                                        class="spreadsheet-item"
                                        on:click=move |_| on_spreadsheet_select(id.clone())
                                    >
                                        {name}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                }
            })}

            <div class="upload-section">
                <div class="upload-area">
                    <h3>"スキーマ (任意)"</h3>
                    <p>"書式定義JSONをアップロード"</p>
                    <input type="file" accept=".json" on:change=on_schema_change />
                    {move || schema.get().map(|s| view! {
                        <p class="status success">"スキーマ: " {s.name}</p>
                    })}
                </div>

                <div class="upload-area">
                    <h3>"ローカルJSON"</h3>
                    <p>"スプレッドシートJSONをアップロード"</p>
                    <input type="file" accept=".json" on:change=on_data_change />
                </div>
            </div>

            <div class="mode-toggle">
                <label>
                    <input
                        type="checkbox"
                        checked=move || use_auto_detect.get()
                        on:change=move |ev| {
                            set_use_auto_detect.set(event_target_checked(&ev));
                        }
                    />
                    " 自動検出モード"
                </label>
            </div>

            {move || spreadsheet.get().map(|data| {
                let sheets: Vec<String> = data.sheets.keys().cloned().collect();
                view! {
                    <div class="sheet-preview">
                        <h2>{data.spreadsheet_name.clone()}</h2>
                        <p>"シート一覧:"</p>
                        <div class="sheet-buttons">
                            {sheets.into_iter().map(|name| {
                                let name_clone = name.clone();
                                let name_display = name.clone();
                                view! {
                                    <button on:click=move |_| on_sheet_select(name_clone.clone())>
                                        {name_display}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                }
            })}

            {move || parsed_result.get().map(|result| view! {
                <div class="sheet-preview">
                    <h2>"解析結果: " {result.sheet_name.clone()}</h2>
                    <p class="schema-info">"使用スキーマ: " {result.schema_name.clone()}</p>
                    <div class="field-list">
                        {result.fields.iter().filter(|f| !f.value.is_empty()).map(|field| {
                            let label = field.label.clone();
                            let value = field.value.clone();
                            let row = field.row;
                            let col = field.col;
                            view! {
                                <div class="field-card">
                                    <h3>{label}</h3>
                                    <div class="value">{value}</div>
                                    <small class="position">
                                        "位置: " {row + 1} "行 " {col + 1} "列"
                                    </small>
                                </div>
                            }
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
