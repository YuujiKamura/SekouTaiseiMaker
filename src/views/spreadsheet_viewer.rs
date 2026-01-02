//! スプレッドシートビューワモジュール

use leptos::*;
use crate::models::ViewMode;
use crate::ProjectContext;

// ============================================
// URL処理ヘルパー関数
// ============================================

/// Google DriveファイルIDからプレビューURLを構築
fn build_drive_preview_url(file_id: &str) -> String {
    format!("https://drive.google.com/file/d/{}/preview", file_id)
}

/// Google Sheets URLからスプレッドシートIDを抽出
/// パターン: /spreadsheets/d/{SPREADSHEET_ID}/...
fn extract_spreadsheet_id(url: &str) -> Option<String> {
    url.split_once("/d/")
        .map(|(_, rest)| rest)
        .and_then(|rest| {
            let id = rest.split(|c| c == '/' || c == '?' || c == '#').next()?;
            (!id.is_empty()).then(|| id.to_string())
        })
}

/// URLからgidパラメータを抽出
fn extract_gid(url: &str) -> Option<String> {
    // #gid= または ?gid= または &gid= を探す
    for prefix in ["#gid=", "?gid=", "&gid="] {
        if let Some((_, rest)) = url.split_once(prefix) {
            let gid: String = rest.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if !gid.is_empty() {
                return Some(gid);
            }
        }
    }
    None
}

/// Google Sheets URLからスプレッドシートIDとgidを抽出
fn extract_spreadsheet_info(url: &str) -> Option<(String, Option<String>)> {
    extract_spreadsheet_id(url).map(|id| (id, extract_gid(url)))
}

/// Google Sheets埋め込みURLを構築
fn build_sheets_embed_url(spreadsheet_id: &str, gid: Option<&str>) -> String {
    let base = format!("https://docs.google.com/spreadsheets/d/{}/preview", spreadsheet_id);
    match gid {
        Some(g) => format!("{}?gid={}", base, g),
        None => base,
    }
}

// ============================================
// スプレッドシートビューワコンポーネント
// ============================================

#[component]
pub fn SpreadsheetViewer(
    contractor: String,
    doc_type: String,
    url: String,
    doc_key: String,
    contractor_id: String,
) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");

    // 未使用パラメータ
    let _ = (doc_key, contractor_id);

    let on_back = move |_| {
        ctx.set_view_mode.set(ViewMode::Dashboard);
    };

    // ローカルパス検出（H:\, C:\, /Users/ など）
    let is_local_path = url.contains(":\\") || url.starts_with("/Users/") || url.starts_with("/home/");

    // Google Sheets URLを埋め込み用に変換（堅牢なID抽出方式）
    // rtpof=true がある場合はExcelファイルなのでDrive形式でプレビュー
    let is_excel_compat = url.contains("rtpof=true");
    let embed_url = if is_local_path {
        String::new()
    } else if url.contains("docs.google.com/spreadsheets") {
        extract_spreadsheet_info(&url)
            .map(|(id, gid)| {
                if is_excel_compat {
                    // ExcelファイルはGoogle Driveのプレビューを使用
                    build_drive_preview_url(&id)
                } else {
                    build_sheets_embed_url(&id, gid.as_deref())
                }
            })
            .unwrap_or_else(|| url.clone())
    } else {
        url.clone()
    };

    view! {
        <div class="viewer-container spreadsheet-viewer">
            <div class="viewer-toolbar">
                <button class="back-btn" on:click=on_back>"← 戻る"</button>
                <span class="doc-info">{contractor.clone()}" / "{doc_type.clone()}</span>
                <div class="toolbar-actions">
                    // スプレッドシートのAIチェックは現在未対応
                </div>
            </div>

            <div class="viewer-content">
                {if is_local_path {
                    view! {
                        <div class="local-path-warning">
                            <p class="warning-title">"ローカルファイルはプレビューできません"</p>
                            <p class="warning-path">{url.clone()}</p>
                            <p class="warning-hint">"目次シートのURLをGoogle Drive Web URL形式に変更してください"</p>
                            <p class="warning-example">"例: https://docs.google.com/spreadsheets/d/ファイルID/edit"</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <iframe
                            src=embed_url
                            class="spreadsheet-frame"
                        ></iframe>
                    }.into_view()
                }}
            </div>
        </div>
    }
}
