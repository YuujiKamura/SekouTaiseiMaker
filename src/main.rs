use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{FileReader, HtmlInputElement};
use std::collections::HashMap;

/// 契約書フィールドの定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractField {
    pub name: String,
    pub value: String,
    pub row: usize,
    pub col: usize,
}

/// 解析済み契約書データ
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedContract {
    pub sheet_name: String,
    pub fields: Vec<ContractField>,
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

/// 契約書の書式を解析
fn parse_contract_sheet(sheet_name: &str, data: &[Vec<String>]) -> ParsedContract {
    let mut fields = Vec::new();

    // 契約書の項目パターンを検出
    let field_patterns = [
        ("工事名", "工事名"),
        ("担当工事内容", "担当工事内容"),
        ("工事場所", "工事場所"),
        ("着　工", "着工日"),
        ("完　成", "完成日"),
        ("工事を施工しない日", "休工日"),
        ("工事を施工しない時間帯", "休工時間帯"),
        ("請負代金額", "請負代金額"),
        ("工事価格", "工事価格"),
    ];

    for (row_idx, row) in data.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            for (pattern, field_name) in &field_patterns {
                if cell.contains(pattern) {
                    // 値は次のセルにある可能性が高い
                    let value = row.get(col_idx + 1)
                        .cloned()
                        .unwrap_or_default();

                    if !value.is_empty() {
                        fields.push(ContractField {
                            name: field_name.to_string(),
                            value,
                            row: row_idx,
                            col: col_idx,
                        });
                    }
                }
            }

            // 金額パターン（¥ で始まる）
            if cell.starts_with('¥') {
                fields.push(ContractField {
                    name: format!("金額({}行)", row_idx + 1),
                    value: cell.clone(),
                    row: row_idx,
                    col: col_idx,
                });
            }
        }
    }

    ParsedContract {
        sheet_name: sheet_name.to_string(),
        fields,
    }
}

#[component]
fn App() -> impl IntoView {
    let (spreadsheet, set_spreadsheet) = create_signal(None::<SpreadsheetData>);
    let (selected_sheet, set_selected_sheet) = create_signal(None::<String>);
    let (parsed_contract, set_parsed_contract) = create_signal(None::<ParsedContract>);

    let on_file_change = move |ev: web_sys::Event| {
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
                                    set_selected_sheet.set(None);
                                    set_parsed_contract.set(None);
                                }
                                Err(e) => {
                                    web_sys::console::log_1(&format!("Parse error: {}", e).into());
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

    let on_sheet_select = move |sheet_name: String| {
        set_selected_sheet.set(Some(sheet_name.clone()));

        if let Some(data) = spreadsheet.get() {
            if let Some(sheet) = data.sheets.get(&sheet_name) {
                let parsed = parse_contract_sheet(&sheet_name, &sheet.data);
                set_parsed_contract.set(Some(parsed));
            }
        }
    };

    view! {
        <div class="container">
            <h1>"施工体制メーカー - シート解析"</h1>

            <div class="upload-area">
                <p>"JSONファイルをアップロード"</p>
                <input type="file" accept=".json" on:change=on_file_change />
            </div>

            {move || spreadsheet.get().map(|data| {
                let sheets: Vec<String> = data.sheets.keys().cloned().collect();
                view! {
                    <div class="sheet-preview">
                        <h2>{data.spreadsheet_name.clone()}</h2>
                        <p>"シート一覧:"</p>
                        <div style="display: flex; gap: 8px; flex-wrap: wrap; margin: 16px 0;">
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

            {move || parsed_contract.get().map(|contract| view! {
                <div class="sheet-preview">
                    <h2>"解析結果: " {contract.sheet_name.clone()}</h2>
                    <div class="field-list">
                        {contract.fields.iter().map(|field| {
                            let name = field.name.clone();
                            let value = field.value.clone();
                            let row = field.row;
                            let col = field.col;
                            view! {
                                <div class="field-card">
                                    <h3>{name}</h3>
                                    <div class="value">{value}</div>
                                    <small style="color: #999;">
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
