//! スプレッドシートビューワモジュール
//!
//! ## 変更履歴
//! - 2026-01-02: 工事名（projectName）をAIチェックURLに追加（バリデーション用）
//! - 2026-01-02: Excelファイル判定（isExcel, fileId）をAIチェックURLに追加
//! - 2026-01-02: AIチェック機能追加（プレビュー画面からSpreadsheetCheckerを呼び出し）
//!
//! ## 既知の動作
//! ブラウザコンソールに以下のCSP違反エラーが表示されることがありますが、これは
//! Googleのセキュリティポリシーによるもので、アプリケーションの動作には影響しません：
//! - "Framing 'https://drive.google.com/' violates Content Security Policy"
//! - "Framing 'https://accounts.google.com/' violates Content Security Policy"
//!
//! Google Sheets/Driveの `/preview` URLは iframe埋め込み用に設計されていますが、
//! 認証関連のサブフレームはGoogleのCSPによりブロックされます。

use leptos::*;
use crate::models::ViewMode;
use crate::ProjectContext;
use crate::utils::gas::get_gas_url;

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
    let (ai_check_mode, set_ai_check_mode) = create_signal(false);

    let on_back = {
        let set_ai_check_mode = set_ai_check_mode.clone();
        move |_| {
            if ai_check_mode.get() {
                set_ai_check_mode.set(false);
            } else {
                ctx.set_view_mode.set(ViewMode::Dashboard);
            }
        }
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

    // AIチェック用のURL構築
    let spreadsheet_info = extract_spreadsheet_info(&url);
    let gas_url = get_gas_url().unwrap_or_default();
    // 工事名を取得（事業所名バリデーション用）
    let project_name = ctx.project.get().map(|p| p.project_name.clone()).unwrap_or_default();
    let ai_check_url = spreadsheet_info.as_ref().map(|(id, gid)| {
        let mut check_url = format!(
            "editor/index.html?mode=spreadsheet-check&spreadsheetId={}&docType={}&contractor={}&gasUrl={}&contractorId={}&docKey={}&projectName={}",
            js_sys::encode_uri_component(id),
            js_sys::encode_uri_component(&doc_type),
            js_sys::encode_uri_component(&contractor),
            js_sys::encode_uri_component(&gas_url),
            js_sys::encode_uri_component(&contractor_id),
            js_sys::encode_uri_component(&doc_key),
            js_sys::encode_uri_component(&project_name)
        );
        if let Some(g) = gid {
            check_url.push_str(&format!("&gid={}", js_sys::encode_uri_component(g)));
        }
        // Excelファイルの場合はfileIdとフラグを追加
        if is_excel_compat {
            check_url.push_str(&format!("&isExcel=true&fileId={}", js_sys::encode_uri_component(id)));
        }
        check_url
    });

    let can_ai_check = spreadsheet_info.is_some() && !gas_url.is_empty();
    let ai_check_url_clone = ai_check_url.clone();

    view! {
        <div class="viewer-container spreadsheet-viewer">
            <div class="viewer-toolbar">
                <button class="back-btn" on:click=on_back>
                    {move || if ai_check_mode.get() { "← プレビューに戻る" } else { "← 戻る" }}
                </button>
                <span class="doc-info">{contractor.clone()}" / "{doc_type.clone()}</span>
                <div class="toolbar-actions">
                    {move || if !ai_check_mode.get() && can_ai_check {
                        view! {
                            <button
                                class="ai-check-btn"
                                on:click=move |_| set_ai_check_mode.set(true)
                            >
                                "🤖 AIチェック"
                            </button>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }}
                </div>
            </div>

            <div class="viewer-content">
                {move || if ai_check_mode.get() {
                    // AIチェックモード
                    if let Some(ref check_url) = ai_check_url_clone {
                        view! {
                            <iframe
                                src=check_url.clone()
                                class="ai-check-frame"
                            ></iframe>
                        }.into_view()
                    } else {
                        view! {
                            <div class="error-message">"AIチェックURLの構築に失敗しました"</div>
                        }.into_view()
                    }
                } else if is_local_path {
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
                            src=embed_url.clone()
                            class="spreadsheet-frame"
                        ></iframe>
                    }.into_view()
                }}
            </div>
        </div>
    }
}
