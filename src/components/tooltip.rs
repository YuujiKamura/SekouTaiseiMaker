//! チェック結果ツールチップコンポーネント
//!
//! エントリーをホバーした時にチェック結果を表示するツールチップ

use crate::models::*;
use crate::ProjectContext;
use leptos::*;

/// チェック結果ツールチップ（1秒ホバーで表示）
#[component]
pub fn CheckResultTooltip() -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");
    let tooltip_state = ctx.check_result_tooltip;

    view! {
        {move || {
            let state = tooltip_state.get();
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
