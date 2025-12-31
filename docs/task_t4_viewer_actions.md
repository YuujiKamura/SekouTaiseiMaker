# Task T4: ビューワアクションUI (Rust)

## 概要
PdfViewer/SpreadsheetViewerにOCR実行ボタン、AIチェックボタン、結果表示パネルを追加。

## 修正ファイル
- `src/main.rs`
- `style.css`

## 前提条件
- T1 (データ構造拡張) 完了
- T2 (API通信モジュール) 完了

## 修正内容

### 1. ViewModeにチェック状態を追加

```rust
#[derive(Clone, PartialEq)]
pub enum ViewMode {
    Dashboard,
    OcrViewer,
    PdfViewer {
        contractor: String,
        doc_type: String,
        url: String,
        doc_key: String,         // 追加: 書類キー（結果保存用）
        contractor_id: String,   // 追加: 業者ID（結果保存用）
    },
    SpreadsheetViewer {
        contractor: String,
        doc_type: String,
        url: String,
        doc_key: String,
        contractor_id: String,
    },
}
```

### 2. ContractorCardのクリック処理を修正

```rust
// ContractorCard内のon_doc_click
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
                    doc_key: key_click.clone(),           // 追加
                    contractor_id: contractor_id_click.clone(), // 追加
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
                if let Some(window) = web_sys::window() {
                    let _ = window.open_with_url_and_target(u, "_blank");
                }
            }
        }
    }
};
```

### 3. PdfViewerを修正

```rust
#[component]
fn PdfViewer(
    contractor: String,
    doc_type: String,
    url: String,
    doc_key: String,
    contractor_id: String,
) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");

    // ローカル状態
    let (checking, set_checking) = create_signal(false);
    let (check_result, set_check_result) = create_signal(None::<CheckResultData>);
    let (error_msg, set_error_msg) = create_signal(None::<String>);

    let on_back = move |_| {
        ctx.set_view_mode.set(ViewMode::Dashboard);
    };

    // チェック実行
    let url_for_check = url.clone();
    let doc_type_for_check = doc_type.clone();
    let contractor_for_check = contractor.clone();
    let doc_key_for_save = doc_key.clone();
    let contractor_id_for_save = contractor_id.clone();

    let on_check = move |_| {
        let url = url_for_check.clone();
        let doc_type = doc_type_for_check.clone();
        let contractor = contractor_for_check.clone();
        let doc_key = doc_key_for_save.clone();
        let contractor_id = contractor_id_for_save.clone();
        let set_project = ctx.set_project;

        set_checking.set(true);
        set_error_msg.set(None);

        spawn_local(async move {
            let request = CheckRequest {
                url,
                doc_type,
                contractor,
            };

            match call_check_api(request).await {
                Ok(result) => {
                    set_check_result.set(Some(result.clone()));

                    // プロジェクトデータを更新
                    set_project.update(|p| {
                        if let Some(project) = p {
                            if let Some(c) = project.contractors.iter_mut()
                                .find(|c| c.id == contractor_id)
                            {
                                if let Some(doc) = c.docs.get_mut(&doc_key) {
                                    doc.check_result = Some(result);
                                    doc.last_checked = Some(
                                        js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default()
                                    );
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    set_error_msg.set(Some(e));
                }
            }
            set_checking.set(false);
        });
    };

    // Google Drive URLをプレビュー用に変換
    let preview_url = if url.contains("drive.google.com/file/d/") {
        url.replace("/view", "/preview")
    } else if url.contains("drive.google.com") && !url.contains("/preview") {
        format!("{}/preview", url.trim_end_matches('/'))
    } else {
        url.clone()
    };

    view! {
        <div class="viewer-container pdf-viewer">
            <div class="viewer-header">
                <button class="back-button" on:click=on_back>
                    "← 戻る"
                </button>
                <div class="doc-title">
                    <span class="contractor-name">{contractor.clone()}</span>
                    <span class="doc-type">{doc_type.clone()}</span>
                </div>
                <a class="external-link" href=url.clone() target="_blank" rel="noopener">
                    "新規タブで開く ↗"
                </a>
            </div>

            // アクションバー（新規追加）
            <div class="viewer-actions">
                <button
                    class="action-btn check-btn"
                    on:click=on_check
                    disabled=move || checking.get() || !ctx.api_connected.get()
                >
                    {move || if checking.get() { "チェック中..." } else { "AIチェック" }}
                </button>

                {move || (!ctx.api_connected.get()).then(|| view! {
                    <span class="api-warning">"※サーバー未接続"</span>
                })}
            </div>

            // エラー表示
            {move || error_msg.get().map(|e| view! {
                <div class="error-message">{e}</div>
            })}

            // チェック結果パネル
            {move || check_result.get().map(|result| view! {
                <CheckResultPanel result=result />
            })}

            <div class="viewer-content">
                <iframe
                    src=preview_url
                    class="pdf-frame"
                ></iframe>
            </div>
        </div>
    }
}
```

### 4. CheckResultPanelコンポーネントを追加

```rust
#[component]
fn CheckResultPanel(result: CheckResultData) -> impl IntoView {
    let status_class = match result.status.as_str() {
        "ok" => "status-ok",
        "warning" => "status-warning",
        "error" => "status-error",
        _ => "status-unknown",
    };

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

            {(!result.items.is_empty()).then(|| view! {
                <div class="result-items">
                    <h4>"チェック項目"</h4>
                    <ul>
                        {result.items.into_iter().map(|item| {
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
            })}

            {(!result.missing_fields.is_empty()).then(|| view! {
                <div class="missing-fields">
                    <h4>"未記入項目"</h4>
                    <ul>
                        {result.missing_fields.into_iter().map(|field| view! {
                            <li>
                                <span class="field-name">{field.field}</span>
                                <span class="field-location">"（"{field.location}"）"</span>
                            </li>
                        }).collect_view()}
                    </ul>
                </div>
            })}
        </div>
    }
}
```

### 5. style.css に追加

```css
/* ビューワアクションバー */
.viewer-actions {
    display: flex;
    gap: 1rem;
    padding: 0.5rem 1rem;
    background: #f5f5f5;
    border-bottom: 1px solid #ddd;
    align-items: center;
}

.action-btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: bold;
}

.check-btn {
    background: #4CAF50;
    color: white;
}

.check-btn:disabled {
    background: #ccc;
    cursor: not-allowed;
}

.api-warning {
    color: #f44336;
    font-size: 0.9rem;
}

.error-message {
    background: #ffebee;
    color: #c62828;
    padding: 0.5rem 1rem;
    margin: 0.5rem 1rem;
    border-radius: 4px;
}

/* チェック結果パネル */
.check-result-panel {
    margin: 0.5rem 1rem;
    padding: 1rem;
    border-radius: 8px;
    border-left: 4px solid;
}

.check-result-panel.status-ok {
    background: #e8f5e9;
    border-color: #4CAF50;
}

.check-result-panel.status-warning {
    background: #fff3e0;
    border-color: #ff9800;
}

.check-result-panel.status-error {
    background: #ffebee;
    border-color: #f44336;
}

.result-header {
    display: flex;
    gap: 1rem;
    align-items: center;
    margin-bottom: 0.5rem;
}

.result-status {
    font-weight: bold;
    font-size: 1.1rem;
}

.result-items ul,
.missing-fields ul {
    list-style: none;
    padding-left: 0;
    margin: 0.5rem 0;
}

.result-items li,
.missing-fields li {
    padding: 0.25rem 0;
}

.item-icon {
    margin-right: 0.5rem;
}

.item-ok { color: #4CAF50; }
.item-warning { color: #ff9800; }
.item-error { color: #f44336; }
```

## テスト方法

```bash
# 1. Pythonサーバー起動
cd scripts && python gemini_server.py

# 2. WASMビルド＆起動
trunk serve

# 3. ダッシュボードから書類をクリック
# 4. 「AIチェック」ボタンをクリック
# 5. 結果パネルが表示されることを確認
```

## 依存関係
- T1 (データ構造拡張) 完了後
- T2 (API通信モジュール) 完了後
