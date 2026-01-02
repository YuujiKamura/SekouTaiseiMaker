//! 業者カードコンポーネント

use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::models::{Contractor, DocFileType, ViewMode, detect_file_type};
use crate::{ContextMenuState, ProjectContext};

/// 業者カードコンポーネント
/// 業者ごとの書類状況を表示し、クリックでドキュメントビューアを開く
#[component]
pub fn ContractorCard(contractor: Contractor) -> impl IntoView {
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

                    // チェック結果からバッジを決定
                    let check_badge = status.check_result.as_ref().map(|r| {
                        match r.status.as_str() {
                            "ok" => ("✓", "badge-ok", "チェック済み"),
                            "warning" => ("⚠", "badge-warning", "要確認"),
                            "error" => ("!", "badge-error", "要対応"),
                            _ => ("?", "badge-unknown", "不明"),
                        }
                    });

                    // ファイルタイプバッジを決定
                    let file_type_badge = url.as_ref().map(|u| {
                        match detect_file_type(u) {
                            DocFileType::Pdf => ("PDF", "file-badge-pdf"),
                            DocFileType::Image => ("IMG", "file-badge-img"),
                            DocFileType::GoogleSpreadsheet => ("シート", "file-badge-sheet"),
                            DocFileType::Excel => ("Excel", "file-badge-excel"),
                            DocFileType::GoogleDoc => ("Doc", "file-badge-doc"),
                            DocFileType::Unknown => ("", ""),
                        }
                    }).filter(|(label, _)| !label.is_empty());

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

                            // ファイルタイプバッジ
                            {file_type_badge.map(|(label, class)| view! {
                                <span class=format!("file-type-badge {}", class)>
                                    {label}
                                </span>
                            })}

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
