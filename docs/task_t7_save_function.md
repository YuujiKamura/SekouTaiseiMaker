# Task T7: 保存機能拡張 (Rust)

## 概要
チェック結果を含むプロジェクトデータの保存・読み込み機能を拡張。

## 修正ファイル
- `src/main.rs`

## 前提条件
- T1 (データ構造拡張) 完了

## 修正内容

### 1. 自動保存の実装

チェック結果が更新された際にLocalStorageへ自動保存する。

```rust
// ProjectContext に自動保存トリガーを追加
// App()内で、projectが変更されたら自動保存

// プロジェクト変更時の自動保存effect
create_effect(move |_| {
    if let Some(p) = project.get() {
        save_to_cache(&p);
    }
});
```

### 2. JSONエクスポート機能の修正

エクスポート時にチェック結果も含まれるように（既存構造体拡張で自動対応）。

```rust
// 既存のエクスポート関数（変更不要だが確認）
fn export_project_json(project: &ProjectData) -> String {
    serde_json::to_string_pretty(project).unwrap_or_default()
}
```

### 3. チェック結果クリア機能

必要に応じて、チェック結果のみをクリアする機能を追加。

```rust
/// 全書類のチェック結果をクリア
fn clear_all_check_results(project: &mut ProjectData) {
    for contractor in &mut project.contractors {
        for (_, doc) in &mut contractor.docs {
            doc.check_result = None;
            doc.last_checked = None;
        }
    }
}

/// 特定の書類のチェック結果をクリア
fn clear_check_result(
    project: &mut ProjectData,
    contractor_id: &str,
    doc_key: &str,
) {
    if let Some(contractor) = project.contractors.iter_mut()
        .find(|c| c.id == contractor_id)
    {
        if let Some(doc) = contractor.docs.get_mut(doc_key) {
            doc.check_result = None;
            doc.last_checked = None;
        }
    }
}
```

### 4. メニューにチェック結果クリアオプション追加

```rust
// メニュー内に追加
<button class="menu-item" on:click=move |_| {
    set_menu_open.set(false);
    set_project.update(|p| {
        if let Some(project) = p {
            clear_all_check_results(project);
        }
    });
}>
    "チェック結果をクリア"
</button>
```

### 5. ダウンロード時のファイル名にタイムスタンプ

```rust
fn download_project_json(project: &ProjectData) {
    let json = serde_json::to_string_pretty(project).unwrap_or_default();

    // タイムスタンプ付きファイル名
    let timestamp = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
        .replace(":", "-")
        .split(".")
        .next()
        .unwrap_or("unknown")
        .to_string();

    let filename = format!(
        "{}_{}.json",
        project.project_name.replace(" ", "_"),
        timestamp
    );

    // Blob作成とダウンロード
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            let blob_parts = js_sys::Array::new();
            blob_parts.push(&JsValue::from_str(&json));

            let mut options = web_sys::BlobPropertyBag::new();
            options.type_("application/json");

            if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
                if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                    if let Ok(a) = document.create_element("a") {
                        let _ = a.set_attribute("href", &url);
                        let _ = a.set_attribute("download", &filename);
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&a);
                            if let Some(html_a) = a.dyn_ref::<web_sys::HtmlElement>() {
                                html_a.click();
                            }
                            let _ = body.remove_child(&a);
                        }
                        let _ = web_sys::Url::revoke_object_url(&url);
                    }
                }
            }
        }
    }
}
```

### 6. 保存状態インジケーター

```rust
// ヘッダーに保存状態表示
<div class="save-indicator">
    {move || {
        if project.get().is_some() {
            view! { <span class="saved">"💾 保存済み"</span> }.into_view()
        } else {
            view! { <span class="no-data">"データなし"</span> }.into_view()
        }
    }}
</div>
```

### 7. style.css

```css
/* 保存インジケーター */
.save-indicator {
    font-size: 0.85rem;
    padding: 0.25rem 0.5rem;
}

.save-indicator .saved {
    color: #4CAF50;
}

.save-indicator .no-data {
    color: #999;
}

.save-indicator .unsaved {
    color: #ff9800;
}
```

## テスト方法

```bash
trunk serve

# 1. プロジェクトを読み込み
# 2. 書類のチェックを実行
# 3. ページをリロード → チェック結果が保持されていることを確認
# 4. JSONエクスポート → チェック結果が含まれていることを確認
# 5. メニューから「チェック結果をクリア」→ バッジが消えることを確認
```

## 依存関係
- T1 (データ構造拡張) 完了後
- 他タスクと並列実行可能
