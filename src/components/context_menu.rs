//! コンテキストメニューコンポーネント
//!
//! 右クリック/ロングプレスで表示される操作選択メニュー

use leptos::*;
use crate::{ContextMenuState, ProjectContext, CheckMode};
use crate::models::{ViewMode, DocFileType, detect_file_type};
use crate::utils::gas::get_gas_url;

/// コンテキストメニュー（操作選択）
#[component]
pub fn ContextMenu() -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");
    let menu_state = ctx.context_menu;
    let set_menu_state = ctx.set_context_menu;
    let set_check_result_tooltip = ctx.set_check_result_tooltip;
    let check_results = ctx.check_results;
    let check_mode = ctx.check_mode;

    // メニューを閉じる
    let close_menu = move |_| {
        set_menu_state.set(ContextMenuState::default());
    };

    // チェック結果を表示
    let show_check_result = move |_| {
        let state = menu_state.get();
        // ツールチップを表示してチェック結果を見せる
        set_check_result_tooltip.set(crate::CheckResultTooltipState {
            visible: true,
            x: state.x,
            y: state.y,
            contractor_name: state.contractor_name.clone(),
            doc_key: state.doc_key.clone(),
            doc_label: state.doc_label.clone(),
            check_result: None, // 個別のAIチェック結果は別途取得が必要
            last_checked: None,
            hover_timer_id: None,
        });
        set_menu_state.set(ContextMenuState::default());
    };

    view! {
        {move || {
            let state = menu_state.get();
            if !state.visible {
                return view! { <></> }.into_view();
            }

            // 画面内に収まるように位置調整
            let window = web_sys::window().unwrap();
            let vw = window.inner_width().unwrap().as_f64().unwrap_or(800.0) as i32;
            let vh = window.inner_height().unwrap().as_f64().unwrap_or(600.0) as i32;
            let menu_width = 200;
            let menu_height = 150;

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

            // このエントリーにチェック結果があるか確認
            let has_check_result = check_mode.get() != CheckMode::None &&
                check_results.get().iter().any(|r| r.contractor_name == state.contractor_name);

            view! {
                // オーバーレイ（メニュー外クリックで閉じる）
                <div class="context-menu-overlay" on:click=close_menu></div>

                <div
                    class="context-menu"
                    style=format!("left: {}px; top: {}px;", x, y)
                >
                    <div class="context-menu-header">
                        <span class="menu-contractor">{state.contractor_name.clone()}</span>
                        <span class="menu-doc">{state.doc_label.clone()}</span>
                    </div>

                    <div class="context-menu-items">
                        // チェック結果表示（結果がある場合のみ）
                        {has_check_result.then(|| view! {
                            <button class="menu-item" on:click=show_check_result>
                                <span class="menu-icon">"📋"</span>
                                <span class="menu-label">"チェック結果を表示"</span>
                            </button>
                        })}

                        // 開く（URLがある場合）
                        {state.url.is_some().then(|| {
                            let url = state.url.clone().unwrap_or_default();
                            let contractor = state.contractor_name.clone();
                            let doc_type = state.doc_label.clone();
                            let doc_key = state.doc_key.clone();
                            let contractor_id = state.contractor_id.clone();
                            let set_view_mode = ctx.set_view_mode;
                            let set_menu = set_menu_state.clone();

                            let set_tooltip = ctx.set_check_result_tooltip;
                            let on_open = move |_| {
                                // クリック時にホバー状態をリセット
                                set_tooltip.set(crate::CheckResultTooltipState::default());
                                set_view_mode.set(ViewMode::PdfViewer {
                                    contractor: contractor.clone(),
                                    doc_type: doc_type.clone(),
                                    url: url.clone(),
                                    doc_key: doc_key.clone(),
                                    contractor_id: contractor_id.clone(),
                                });
                                set_menu.set(ContextMenuState::default());
                            };

                            view! {
                                <button class="menu-item" on:click=on_open>
                                    <span class="menu-icon">"📄"</span>
                                    <span class="menu-label">"開く"</span>
                                </button>
                            }
                        })}

                        // AI自動修正（スプレッドシート/Excelの場合）
                        {state.url.as_ref().and_then(|url| {
                            let file_type = detect_file_type(url);
                            match file_type {
                                DocFileType::GoogleSpreadsheet | DocFileType::Excel => {
                                    let url = url.clone();
                                    let contractor = state.contractor_name.clone();
                                    let doc_type = state.doc_label.clone();
                                    let doc_key = state.doc_key.clone();
                                    let contractor_id = state.contractor_id.clone();
                                    let set_view_mode = ctx.set_view_mode;
                                    let set_menu = set_menu_state.clone();
                                    let set_tooltip = ctx.set_check_result_tooltip;

                                    let on_auto_fix = move |_| {
                                        set_tooltip.set(crate::CheckResultTooltipState::default());
                                        set_view_mode.set(ViewMode::SpreadsheetViewer {
                                            contractor: contractor.clone(),
                                            doc_type: doc_type.clone(),
                                            url: url.clone(),
                                            doc_key: doc_key.clone(),
                                            contractor_id: contractor_id.clone(),
                                            auto_fix: true,
                                        });
                                        set_menu.set(ContextMenuState::default());
                                    };

                                    Some(view! {
                                        <button class="menu-item menu-item-autofix" on:click=on_auto_fix>
                                            <span class="menu-icon">"🔧"</span>
                                            <span class="menu-label">"AI自動修正"</span>
                                        </button>
                                    })
                                }
                                _ => None
                            }
                        })}

                        // 修正版を採用（URLがある場合は常に表示）
                        {state.url.as_ref().map(|url| {
                            let url = url.clone();
                            let doc_key = state.doc_key.clone();
                            let contractor_id = state.contractor_id.clone();
                            let set_menu = set_menu_state.clone();
                            let set_project = ctx.set_project;
                            let project = ctx.project;

                            let on_adopt_fixed = move |_| {
                                let url = url.clone();
                                let doc_key = doc_key.clone();
                                let contractor_id = contractor_id.clone();
                                let set_project = set_project.clone();
                                let project = project.clone();

                                // メニューを閉じる
                                set_menu.set(ContextMenuState::default());

                                // 非同期で修正版を検索・採用
                                spawn_local(async move {
                                    if let Err(e) = adopt_fixed_version(&url, &contractor_id, &doc_key, set_project, project).await {
                                        web_sys::window()
                                            .and_then(|w| w.alert_with_message(&format!("修正版の採用に失敗しました: {}", e)).ok());
                                    }
                                });
                            };

                            view! {
                                <button class="menu-item menu-item-adopt" on:click=on_adopt_fixed>
                                    <span class="menu-icon">"📥"</span>
                                    <span class="menu-label">"修正版を採用"</span>
                                </button>
                            }
                        })}

                    </div>
                </div>
            }.into_view()
        }}
    }
}

/// URLからファイルIDを抽出
fn extract_file_id(url: &str) -> Option<String> {
    // Google Drive URL: https://drive.google.com/file/d/{fileId}/view
    if let Some(start) = url.find("/d/") {
        let rest = &url[start + 3..];
        if let Some(end) = rest.find('/') {
            return Some(rest[..end].to_string());
        }
        // /view がない場合
        return Some(rest.to_string());
    }
    None
}

/// 修正版ファイルを検索して採用
async fn adopt_fixed_version(
    url: &str,
    contractor_id: &str,
    doc_key: &str,
    set_project: WriteSignal<Option<crate::models::ProjectData>>,
    project: ReadSignal<Option<crate::models::ProjectData>>,
) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let gas_url = get_gas_url().ok_or("GAS URLが設定されていません")?;
    let file_id = extract_file_id(url).ok_or("ファイルIDを抽出できません")?;

    web_sys::console::log_1(&format!("[adopt_fixed_version] url: {}", url).into());
    web_sys::console::log_1(&format!("[adopt_fixed_version] file_id: {}", file_id).into());

    // 修正版ファイルを検索
    let latest_url = format!(
        "{}?action=getLatestFile&fileId={}",
        gas_url,
        js_sys::encode_uri_component(&file_id)
    );

    web_sys::console::log_1(&format!("[adopt_fixed_version] latest_url: {}", latest_url).into());

    let window = web_sys::window().ok_or("window not found")?;
    let resp = JsFuture::from(window.fetch_with_str(&latest_url))
        .await
        .map_err(|e| format!("fetch error: {:?}", e))?;

    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "Response cast error")?;
    let json = JsFuture::from(resp.json().map_err(|_| "json() error")?)
        .await
        .map_err(|e| format!("json parse error: {:?}", e))?;

    let latest_data: serde_json::Value = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("deserialize error: {:?}", e))?;

    web_sys::console::log_1(&format!("[adopt_fixed_version] response: {:?}", latest_data).into());

    if let Some(error) = latest_data.get("error").and_then(|v| v.as_str()) {
        return Err(error.to_string());
    }

    let is_fixed = latest_data.get("isFixedVersion").and_then(|v| v.as_bool()).unwrap_or(false);
    web_sys::console::log_1(&format!("[adopt_fixed_version] isFixedVersion: {}", is_fixed).into());

    if !is_fixed {
        return Err("修正版ファイルが見つかりません。\nダウンロードしたファイルをGoogle Driveの同じフォルダに保存してください。".to_string());
    }

    let new_file_id = latest_data.get("fileId").and_then(|v| v.as_str())
        .ok_or("新しいファイルIDが見つかりません")?;
    let new_file_name = latest_data.get("fileName").and_then(|v| v.as_str())
        .unwrap_or("修正版ファイル");

    // ProjectDataのURLを更新
    let update_url = format!(
        "{}?action=updateDocUrl&contractorId={}&docKey={}&newFileId={}",
        gas_url,
        js_sys::encode_uri_component(contractor_id),
        js_sys::encode_uri_component(doc_key),
        js_sys::encode_uri_component(new_file_id)
    );

    web_sys::console::log_1(&format!("[adopt_fixed_version] update_url: {}", update_url).into());
    web_sys::console::log_1(&format!("[adopt_fixed_version] contractor_id: {}, doc_key: {}", contractor_id, doc_key).into());

    let resp = JsFuture::from(window.fetch_with_str(&update_url))
        .await
        .map_err(|e| format!("fetch error: {:?}", e))?;

    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "Response cast error")?;

    // レスポンスのテキストを取得してログ出力
    let text = JsFuture::from(resp.text().map_err(|_| "text() error")?)
        .await
        .map_err(|e| format!("text parse error: {:?}", e))?;

    let text_str = text.as_string().unwrap_or_default();
    web_sys::console::log_1(&format!("[adopt_fixed_version] update response text: {}", text_str).into());

    let update_data: serde_json::Value = serde_json::from_str(&text_str)
        .map_err(|e| format!("JSON parse error: {:?}, response: {}", e, text_str))?;

    web_sys::console::log_1(&format!("[adopt_fixed_version] update_data: {:?}", update_data).into());

    if let Some(error) = update_data.get("error").and_then(|v| v.as_str()) {
        return Err(error.to_string());
    }

    // ローカルのProjectデータも更新
    if let Some(mut proj) = project.get() {
        // ファイル名から適切なURL形式を決定（大文字小文字無視）
        let file_name_lower = new_file_name.to_lowercase();
        let is_excel = file_name_lower.ends_with(".xlsx") || file_name_lower.ends_with(".xls");
        web_sys::console::log_1(&format!("[adopt_fixed_version] file_name: {}, is_excel: {}", new_file_name, is_excel).into());

        let new_url = if is_excel {
            // type=xlsxを追加してファイルタイプ判定で正しくExcelと認識させる
            format!("https://drive.google.com/file/d/{}/view?usp=drivesdk&type=xlsx", new_file_id)
        } else {
            format!("https://docs.google.com/spreadsheets/d/{}/edit?usp=drivesdk", new_file_id)
        };
        web_sys::console::log_1(&format!("[adopt_fixed_version] new_url: {}", new_url).into());
        for contractor in proj.contractors.iter_mut() {
            if contractor.id == contractor_id {
                if let Some(doc) = contractor.docs.get_mut(doc_key) {
                    doc.url = Some(new_url.clone());
                }
                break;
            }
        }
        set_project.set(Some(proj));
    }

    // 成功メッセージ
    window.alert_with_message(&format!("修正版を採用しました: {}", new_file_name)).ok();

    Ok(())
}
