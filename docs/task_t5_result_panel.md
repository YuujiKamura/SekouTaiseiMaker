# Task T5: 結果パネルUI (Rust)

## 概要
チェック結果を見やすく表示するパネルUIの詳細実装。
T4で基本形を作成済みの場合は、このタスクで拡張・改善を行う。

## 修正ファイル
- `src/main.rs`
- `style.css`

## 前提条件
- T1 (データ構造拡張) 完了

## 修正内容

### 1. 結果パネルの拡張版

```rust
#[component]
fn CheckResultPanel(
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
```

### 2. style.css 拡張

```css
/* チェック結果パネル - 詳細スタイル */
.check-result-panel {
    margin: 0.75rem;
    padding: 1rem;
    border-radius: 8px;
    border-left: 4px solid;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    max-height: 40vh;
    overflow-y: auto;
}

.check-result-panel.status-ok {
    background: linear-gradient(135deg, #e8f5e9 0%, #c8e6c9 100%);
    border-color: #4CAF50;
}

.check-result-panel.status-warning {
    background: linear-gradient(135deg, #fff3e0 0%, #ffe0b2 100%);
    border-color: #ff9800;
}

.check-result-panel.status-error {
    background: linear-gradient(135deg, #ffebee 0%, #ffcdd2 100%);
    border-color: #f44336;
}

/* ヘッダー */
.result-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
}

.result-status-badge {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.75rem;
    border-radius: 20px;
    background: rgba(255,255,255,0.7);
}

.status-icon {
    font-size: 1.2rem;
}

.status-label {
    font-weight: bold;
}

.close-btn {
    background: transparent;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    opacity: 0.6;
    padding: 0 0.5rem;
}

.close-btn:hover {
    opacity: 1;
}

/* サマリー */
.result-summary {
    font-size: 1rem;
    margin-bottom: 0.75rem;
    padding: 0.5rem;
    background: rgba(255,255,255,0.5);
    border-radius: 4px;
}

/* 統計バー */
.result-stats {
    display: flex;
    gap: 1rem;
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
}

.stat {
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
}

.stat-ok { background: #c8e6c9; color: #2e7d32; }
.stat-warning { background: #ffe0b2; color: #ef6c00; }
.stat-error { background: #ffcdd2; color: #c62828; }

/* 折りたたみ詳細 */
.result-details,
.missing-fields-details {
    margin-top: 0.5rem;
}

.result-details summary,
.missing-fields-details summary {
    cursor: pointer;
    font-weight: bold;
    padding: 0.5rem;
    background: rgba(255,255,255,0.5);
    border-radius: 4px;
    user-select: none;
}

.result-details summary:hover,
.missing-fields-details summary:hover {
    background: rgba(255,255,255,0.7);
}

/* チェック項目リスト */
.result-items-list,
.missing-fields-list {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0 0 0;
}

.result-item,
.missing-field-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.5rem;
    margin: 0.25rem 0;
    background: rgba(255,255,255,0.5);
    border-radius: 4px;
}

.item-icon,
.field-icon {
    flex-shrink: 0;
    width: 1.5rem;
    text-align: center;
}

.item-message,
.field-name {
    flex: 1;
}

.field-location {
    color: #666;
    font-size: 0.85rem;
}

/* アイテムタイプ別の色 */
.item-ok .item-icon { color: #4CAF50; }
.item-warning .item-icon { color: #ff9800; }
.item-error .item-icon { color: #f44336; }
.item-info .item-icon { color: #2196F3; }

/* 未記入ヘッダー強調 */
.missing-header {
    color: #c62828;
}
```

## テスト方法

```bash
trunk build
# ビューワでチェック結果が正しく表示されることを確認
```

## 依存関係
- T1 (データ構造拡張) 完了後
- T4と並列または後続で実行可能
