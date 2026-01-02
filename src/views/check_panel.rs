//! チェック結果パネルモジュール
//!
//! PDFドキュメントのチェック結果を表示するパネルコンポーネント

use leptos::*;
use crate::models::CheckResultData;
use crate::{CheckMode, CheckStatus, ProjectContext};

// ============================================
// チェック結果パネル (拡張版 T5)
// ============================================

/// 拡張版チェック結果パネルコンポーネント
#[component]
pub fn CheckResultPanel(
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
pub fn CheckResultsPanel() -> impl IntoView {
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
