//! PDFビューワコンポーネント

use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::models::{CheckResultData, ViewMode};
use crate::utils::gas::get_gas_url;
use crate::ProjectContext;

// ============================================
// Google Drive URL解析ヘルパー
// ============================================

/// Google DriveファイルURLからファイルIDを抽出
fn extract_drive_file_id(url: &str) -> Option<String> {
    if let Some(start) = url.find("/d/") {
        let after_d = &url[start + 3..];
        let end = after_d.find('/').unwrap_or(after_d.len());
        let file_id = &after_d[..end];
        // クエリパラメータを除去
        let file_id = file_id.split('?').next().unwrap_or(file_id);
        if !file_id.is_empty() {
            return Some(file_id.to_string());
        }
    }
    None
}

// ============================================
// PDFビューワコンポーネント
// ============================================

#[component]
pub fn PdfViewer(
    contractor: String,
    doc_type: String,
    url: String,
    doc_key: String,
    contractor_id: String,
) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");
    let set_view_mode = ctx.set_view_mode;
    let _api_connected = ctx.api_connected;

    // エラーメッセージ（ローカルファイル用）
    let (_error_msg, _set_error_msg) = create_signal(None::<String>);

    // PDFプリフェッチ（バックグラウンドでキャッシュ）
    {
        let url_for_prefetch = url.clone();
        create_effect(move |_| {
            if let Some(file_id) = extract_drive_file_id(&url_for_prefetch) {
                if let Some(gas_url) = get_gas_url() {
                    // JavaScript の prefetchPdf を呼び出し
                    let _ = js_sys::eval(&format!(
                        "window.prefetchPdf && window.prefetchPdf('{}', '{}')",
                        file_id, gas_url
                    ));
                }
            }
        });
    }

    let on_back = move |_: web_sys::MouseEvent| {
        set_view_mode.set(ViewMode::Dashboard);
    };

    // ローカルパス検出（H:\, C:\, /Users/ など）
    let is_local_path = url.contains(":\\") || url.starts_with("/Users/") || url.starts_with("/home/");

    // React viewer用のiframe URL構築
    let iframe_url = if is_local_path {
        String::new()
    } else {
        let file_id = extract_drive_file_id(&url).unwrap_or_default();
        let gas_url = get_gas_url().unwrap_or_default();
        format!(
            "editor/index.html?mode=view&fileId={}&docType={}&contractor={}&gasUrl={}",
            js_sys::encode_uri_component(&file_id),
            js_sys::encode_uri_component(&doc_type),
            js_sys::encode_uri_component(&contractor),
            js_sys::encode_uri_component(&gas_url)
        )
    };

    let url_display = url.clone();

    // postMessage ハンドラ（viewer-back, viewer-edit, viewer-check）
    {
        let set_view_mode = ctx.set_view_mode.clone();
        let contractor_for_msg = contractor.clone();
        let doc_type_for_msg = doc_type.clone();
        let url_for_msg = url.clone();
        let doc_key_for_msg = doc_key.clone();
        let contractor_id_for_msg = contractor_id.clone();

        create_effect(move |_| {
            let set_view_mode = set_view_mode.clone();
            let contractor = contractor_for_msg.clone();
            let doc_type = doc_type_for_msg.clone();
            let url = url_for_msg.clone();
            let doc_key = doc_key_for_msg.clone();
            let contractor_id = contractor_id_for_msg.clone();

            let handler = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                    if let Some(msg_type) = js_sys::Reflect::get(&data, &"type".into())
                        .ok()
                        .and_then(|v| v.as_string())
                    {
                        match msg_type.as_str() {
                            "viewer-back" => {
                                set_view_mode.set(ViewMode::Dashboard);
                            }
                            "viewer-edit" => {
                                set_view_mode.set(ViewMode::PdfEditor {
                                    contractor: contractor.clone(),
                                    doc_type: doc_type.clone(),
                                    original_url: url.clone(),
                                });
                            }
                            "viewer-check" => {
                                if let Some(file_id) = extract_drive_file_id(&url) {
                                    set_view_mode.set(ViewMode::AiChecker {
                                        contractor: contractor.clone(),
                                        doc_type: doc_type.clone(),
                                        file_id,
                                        doc_key: doc_key.clone(),
                                        contractor_id: contractor_id.clone(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let window = web_sys::window().unwrap();
            let _ = window.add_event_listener_with_callback("message", handler.as_ref().unchecked_ref());
            handler.forget();
        });
    }

    view! {
        <div class="viewer-container pdf-viewer">
            {if is_local_path {
                view! {
                    <div class="local-path-warning">
                        <button class="back-btn" on:click=on_back>"← 戻る"</button>
                        <p class="warning-title">"ローカルファイルはプレビューできません"</p>
                        <p class="warning-path">{url_display}</p>
                        <p class="warning-hint">"目次シートのURLをGoogle Drive Web URL形式に変更してください"</p>
                        <p class="warning-example">"例: https://drive.google.com/file/d/ファイルID/view"</p>
                    </div>
                }.into_view()
            } else {
                view! {
                    <iframe
                        src=iframe_url
                        class="pdf-frame"
                        style="width: 100%; height: 100vh; border: none;"
                    ></iframe>
                }.into_view()
            }}
        </div>
    }
}

// ============================================
// ビューワチェック結果パネル
// ============================================

#[component]
pub fn ViewerCheckResultPanel(result: CheckResultData) -> impl IntoView {
    let status_class = match result.status.as_str() {
        "ok" => "status-ok",
        "warning" => "status-warning",
        "error" => "status-error",
        _ => "status-unknown",
    };

    let result_items = result.items.clone();
    let missing_fields = result.missing_fields.clone();

    view! {
        <div class=format!("check-result-panel {}", status_class)>
            <div class="result-header">
                <span class="result-status">{
                    match result.status.as_str() {
                        "ok" => "✓ OK",
                        "warning" => "⚠ 警告",
                        "error" => "✗ エラー",
                        _ => "? 不明",
                    }
                }</span>
                <span class="result-summary">{result.summary}</span>
            </div>

            {(!result_items.is_empty()).then(|| {
                let items = result_items.clone();
                view! {
                    <div class="result-items">
                        <h4>"チェック項目"</h4>
                        <ul>
                            {items.into_iter().map(|item| {
                                let icon = match item.item_type.as_str() {
                                    "ok" => "✓",
                                    "warning" => "⚠",
                                    "error" => "✗",
                                    _ => "•",
                                };
                                view! {
                                    <li class=format!("item-{}", item.item_type)>
                                        <span class="item-icon">{icon}</span>
                                        <span class="item-message">{item.message}</span>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    </div>
                }
            })}

            {(!missing_fields.is_empty()).then(|| {
                let fields = missing_fields.clone();
                view! {
                    <div class="missing-fields-list">
                        <h4>"未記入項目"</h4>
                        <ul>
                            {fields.into_iter().map(|field| view! {
                                <li>
                                    <span class="field-name">{field.field}</span>
                                    <span class="field-location">"（"{field.location}"）"</span>
                                </li>
                            }).collect_view()}
                        </ul>
                    </div>
                }
            })}
        </div>
    }
}
