//! コンテキストメニューコンポーネント
//!
//! 右クリック/ロングプレスで表示される操作選択メニュー

use leptos::*;
use crate::{ContextMenuState, ProjectContext, CheckMode};

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

                        // PDFで開く（URLがある場合）
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
                                set_view_mode.set(crate::models::ViewMode::PdfViewer {
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

                        // 閉じる
                        <button class="menu-item menu-item-close" on:click=close_menu>
                            <span class="menu-icon">"✕"</span>
                            <span class="menu-label">"閉じる"</span>
                        </button>
                    </div>
                </div>
            }.into_view()
        }}
    }
}
