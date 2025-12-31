# Task T6: ダッシュボード所見表示 (Rust)

## 概要
ダッシュボードの書類リストに、AIチェック結果のバッジ/アイコンを表示する。

## 修正ファイル
- `src/main.rs`
- `style.css`

## 前提条件
- T1 (データ構造拡張) 完了

## 修正内容

### 1. ContractorCard内のDocアイテム表示を修正

```rust
// ContractorCard内のdocsループ部分を修正
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

    let last_checked = status.last_checked.clone();

    // クリック用の変数クローン
    let contractor_name_click = contractor_name.clone();
    let label_click = label.clone();
    let url_click = url.clone();
    let key_click = key.clone();
    let contractor_id_click = contractor.id.clone();
    let set_view_mode = ctx.set_view_mode;

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
        >
            // 書類状態アイコン
            <span class="doc-icon">{if status.status { "✓" } else { "✗" }}</span>

            // 書類名
            <span class=format!("doc-name {}", if has_url { "doc-link" } else { "" })>
                {label.clone()}
            </span>

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
```

### 2. ContractorCardヘッダーにチェック状況サマリーを追加

```rust
#[component]
fn ContractorCard(contractor: Contractor) -> impl IntoView {
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

    // ドキュメントをソート
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
                // ... docs表示 ...
            </div>
        </div>
    }
}
```

### 3. style.css 追加

```css
/* チェックバッジ */
.check-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 50%;
    font-size: 0.8rem;
    font-weight: bold;
    margin-left: 0.25rem;
    flex-shrink: 0;
}

.badge-ok {
    background: #c8e6c9;
    color: #2e7d32;
}

.badge-warning {
    background: #ffe0b2;
    color: #ef6c00;
}

.badge-error {
    background: #ffcdd2;
    color: #c62828;
    animation: pulse 1.5s infinite;
}

@keyframes pulse {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.1); }
}

.badge-unknown {
    background: #e0e0e0;
    color: #616161;
}

/* 最終チェック日時 */
.last-checked {
    font-size: 0.8rem;
    opacity: 0.6;
    margin-left: 0.25rem;
}

/* ヘッダー統計 */
.header-stats {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-left: auto;
}

.checked-stats {
    display: flex;
    gap: 0.25rem;
    font-size: 0.85rem;
}

.stat-error {
    color: #c62828;
    font-weight: bold;
}

.stat-warning {
    color: #ef6c00;
}

.stat-checked {
    color: #666;
}

/* doc-item にバッジ付きの場合のスタイル */
.doc-item.badge-warning {
    border-left: 3px solid #ff9800;
}

.doc-item.badge-error {
    border-left: 3px solid #f44336;
    background: rgba(244, 67, 54, 0.05);
}
```

## テスト方法

```bash
trunk build

# 1. ダッシュボードを開く
# 2. 書類にチェック結果があれば、バッジが表示されることを確認
# 3. 業者カードのヘッダーに集計が表示されることを確認
```

## 依存関係
- T1 (データ構造拡張) 完了後
- T4と並列実行可能
