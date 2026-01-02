//! エディタコンポーネントモジュール
//!
//! ProjectEditor, ContractorEditor, DocEditor を提供

use leptos::*;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use crate::models::{Contractor, DocStatus, ProjectData, DocLink};
use crate::ProjectContext;
use crate::utils::gas::{get_gas_url, save_to_gas};
use crate::utils::cache::save_to_cache;

/// 標準的な書類リスト
pub const STANDARD_DOCS: &[(&str, &str)] = &[
    ("01_建設業許可", "建設業許可"),
    ("02_事業所番号", "事業所番号"),
    ("03_労働保険番号", "労働保険番号"),
    ("041_現場代理人資格", "現場代理人資格"),
    ("042_現場代理人在籍", "現場代理人在籍"),
    ("051_主任技術者資格", "主任技術者資格"),
    ("052_主任技術者在籍", "主任技術者在籍"),
    ("06_法定外労災", "法定外労災"),
    ("07_建退共", "建退共"),
    ("08_作業員名簿", "作業員名簿"),
    ("09_暴対法誓約書", "暴対法誓約書"),
];

/// プロジェクト全体書類の編集用コンポーネント
#[component]
fn ProjectDocEditor<G, F>(
    label: &'static str,
    doc: G,
    on_update: F,
) -> impl IntoView
where
    G: Fn() -> Option<DocLink> + 'static,
    F: Fn(Option<DocLink>) + 'static + Clone,
{
    let initial = doc();
    let (status, set_status) = create_signal(initial.as_ref().map(|d| d.status).unwrap_or(false));
    let (url, set_url) = create_signal(initial.as_ref().and_then(|d| d.url.clone()).unwrap_or_default());

    let on_update_1 = on_update.clone();
    let on_update_2 = on_update;

    view! {
        <div class="project-doc-editor-row">
            <label class="checkbox-label">
                <input type="checkbox"
                    prop:checked=move || status.get()
                    on:change=move |ev| {
                        let new_status = event_target_checked(&ev);
                        set_status.set(new_status);
                        on_update_1(Some(DocLink {
                            name: label.to_string(),
                            url: if url.get().is_empty() { None } else { Some(url.get()) },
                            status: new_status,
                        }));
                    }
                />
                <span class="doc-label">{label}</span>
            </label>
            <input type="text" class="url-input" placeholder="URL"
                prop:value=move || url.get()
                on:input=move |ev| {
                    let new_url = event_target_value(&ev);
                    set_url.set(new_url.clone());
                    on_update_2(Some(DocLink {
                        name: label.to_string(),
                        url: if new_url.is_empty() { None } else { Some(new_url) },
                        status: status.get(),
                    }));
                }
            />
        </div>
    }
}

/// プロジェクト編集コンポーネント
#[component]
pub fn ProjectEditor(project: ProjectData) -> impl IntoView {
    let ctx = use_context::<ProjectContext>().expect("ProjectContext not found");

    // ローカルで編集可能な状態を作成
    let (project_name, set_project_name) = create_signal(project.project_name.clone());
    let (client, set_client) = create_signal(project.client.clone());
    let (period, set_period) = create_signal(project.period.clone());
    let (project_docs, set_project_docs) = create_signal(project.project_docs.clone());
    let (contractors, set_contractors) = create_signal(project.contractors.clone());
    let (contracts, _) = create_signal(project.contracts.clone());

    // 保存状態
    let (saving, set_saving) = create_signal(false);
    let (save_message, set_save_message) = create_signal(None::<String>);

    // 変更を保存（ローカル + GAS）
    let save_changes = move |_| {
        let updated = ProjectData {
            project_name: project_name.get(),
            client: client.get(),
            period: period.get(),
            project_docs: project_docs.get(),
            contractors: contractors.get(),
            contracts: contracts.get(),
        };

        // ローカル状態を更新
        ctx.set_project.set(Some(updated.clone()));
        // キャッシュに保存
        save_to_cache(&updated);

        // GASに保存（接続している場合）
        if get_gas_url().is_some() {
            set_saving.set(true);
            set_save_message.set(None);
            spawn_local(async move {
                match save_to_gas(&updated).await {
                    Ok(_) => {
                        set_save_message.set(Some("保存しました".to_string()));
                    }
                    Err(e) => {
                        set_save_message.set(Some(format!("保存エラー: {}", e)));
                    }
                }
                set_saving.set(false);
            });
        } else {
            set_save_message.set(Some("ローカルに保存しました（シート未接続）".to_string()));
        }
    };

    // 編集を終了
    let exit_edit = move |_| {
        ctx.set_edit_mode.set(false);
    };

    // 業者追加
    let add_contractor = move |_| {
        set_contractors.update(|cs| {
            let new_id = format!("contractor_{}", cs.len() + 1);
            cs.push(Contractor {
                id: new_id,
                name: "新規業者".to_string(),
                role: "".to_string(),
                docs: HashMap::new(),
            });
        });
    };

    // 業者削除
    let delete_contractor = move |idx: usize| {
        set_contractors.update(|cs| {
            if idx < cs.len() {
                cs.remove(idx);
            }
        });
    };

    // 業者更新
    let update_contractor = move |idx: usize, updated: Contractor| {
        set_contractors.update(|cs| {
            if idx < cs.len() {
                cs[idx] = updated;
            }
        });
    };

    view! {
        <div class="project-editor">
            <div class="editor-header">
                <h2>"プロジェクト編集"</h2>
                <div class="editor-actions">
                    <button class="back-btn" on:click=exit_edit>"← 戻る"</button>
                    <button class="save-btn" on:click=save_changes disabled=move || saving.get()>
                        {move || if saving.get() { "保存中..." } else { "変更を保存" }}
                    </button>
                </div>
            </div>
            {move || save_message.get().map(|msg| view! {
                <div class=format!("save-message {}", if msg.contains("エラー") { "error" } else { "success" })>
                    {msg}
                </div>
            })}

            <div class="editor-section">
                <h3>"基本情報"</h3>
                <div class="form-group">
                    <label>"工事名"</label>
                    <input type="text"
                        prop:value=move || project_name.get()
                        on:input=move |ev| set_project_name.set(event_target_value(&ev))
                    />
                </div>
                <div class="form-row">
                    <div class="form-group">
                        <label>"発注者"</label>
                        <input type="text"
                            prop:value=move || client.get()
                            on:input=move |ev| set_client.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="form-group">
                        <label>"工期"</label>
                        <input type="text"
                            prop:value=move || period.get()
                            on:input=move |ev| set_period.set(event_target_value(&ev))
                        />
                    </div>
                </div>
            </div>

            <div class="editor-section">
                <h3>"全体書類"</h3>
                <div class="project-docs-editor">
                    <ProjectDocEditor
                        label="施工体系図"
                        doc=move || project_docs.get().sekou_taikeizu.clone()
                        on_update=move |d| set_project_docs.update(|pd| pd.sekou_taikeizu = d)
                    />
                    <ProjectDocEditor
                        label="施工体制台帳"
                        doc=move || project_docs.get().sekou_taisei_daicho.clone()
                        on_update=move |d| set_project_docs.update(|pd| pd.sekou_taisei_daicho = d)
                    />
                    <ProjectDocEditor
                        label="下請契約書"
                        doc=move || project_docs.get().shitauke_keiyaku.clone()
                        on_update=move |d| set_project_docs.update(|pd| pd.shitauke_keiyaku = d)
                    />
                </div>
            </div>

            <div class="editor-section">
                <div class="section-header">
                    <h3>"業者一覧"</h3>
                    <button class="add-btn" on:click=add_contractor>"+ 業者追加"</button>
                </div>

                <div class="contractors-editor">
                    {move || contractors.get().into_iter().enumerate().map(|(idx, c)| {
                        let update_fn = move |updated: Contractor| update_contractor(idx, updated);
                        let delete_fn = move |_| delete_contractor(idx);
                        view! {
                            <ContractorEditor
                                contractor=c
                                on_update=update_fn
                                on_delete=delete_fn
                            />
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}

/// 業者編集コンポーネント
#[component]
pub fn ContractorEditor<F, D>(
    contractor: Contractor,
    on_update: F,
    on_delete: D,
) -> impl IntoView
where
    F: Fn(Contractor) + 'static + Clone,
    D: Fn(()) + 'static,
{
    let (name, set_name) = create_signal(contractor.name.clone());
    let (role, set_role) = create_signal(contractor.role.clone());
    let (docs, set_docs) = create_signal(contractor.docs.clone());
    let (expanded, set_expanded) = create_signal(false);

    let contractor_id = contractor.id.clone();

    view! {
        <div class="contractor-editor">
            <div class="contractor-editor-header" on:click=move |_| set_expanded.update(|e| *e = !*e)>
                <span class="expand-icon">{move || if expanded.get() { "▼" } else { "▶" }}</span>
                <input type="text" class="name-input"
                    prop:value=move || name.get()
                    on:input={
                        let contractor_id = contractor_id.clone();
                        let on_update = on_update.clone();
                        move |ev| {
                            set_name.set(event_target_value(&ev));
                            on_update(Contractor {
                                id: contractor_id.clone(),
                                name: name.get(),
                                role: role.get(),
                                docs: docs.get(),
                            });
                        }
                    }
                    on:click=move |ev| ev.stop_propagation()
                />
                <input type="text" class="role-input" placeholder="役割"
                    prop:value=move || role.get()
                    on:input={
                        let contractor_id = contractor_id.clone();
                        let on_update = on_update.clone();
                        move |ev| {
                            set_role.set(event_target_value(&ev));
                            on_update(Contractor {
                                id: contractor_id.clone(),
                                name: name.get(),
                                role: role.get(),
                                docs: docs.get(),
                            });
                        }
                    }
                    on:click=move |ev| ev.stop_propagation()
                />
                <button class="delete-btn" on:click=move |ev| {
                    ev.stop_propagation();
                    on_delete(());
                }>"削除"</button>
            </div>

            {move || {
                let is_expanded = expanded.get();
                let on_update = on_update.clone();
                let contractor_id = contractor_id.clone();

                is_expanded.then(|| {
                    let mut doc_list: Vec<_> = docs.get().into_iter().collect();
                    doc_list.sort_by(|a, b| a.0.cmp(&b.0));

                    let on_update_add = on_update.clone();
                    let contractor_id_add = contractor_id.clone();

                    view! {
                        <div class="docs-editor">
                            <div class="docs-header">
                                <span>"書類一覧"</span>
                                <button class="add-btn small" on:click=move |_| {
                                    set_docs.update(|d| {
                                        for (key, _) in STANDARD_DOCS {
                                            if !d.contains_key(*key) {
                                                d.insert(key.to_string(), DocStatus {
                                                    status: false,
                                                    file: None,
                                                    url: None,
                                                    note: Some("要依頼".to_string()),
                                                    valid_from: None,
                                                    valid_until: None,
                                                    check_result: None,
                                                    last_checked: None,
                                                });
                                                break;
                                            }
                                        }
                                    });
                                    on_update_add(Contractor {
                                        id: contractor_id_add.clone(),
                                        name: name.get(),
                                        role: role.get(),
                                        docs: docs.get(),
                                    });
                                }>"+ 書類追加"</button>
                            </div>
                            {doc_list.into_iter().map(|(key, status)| {
                                let key_clone = key.clone();
                                let key_for_delete = key.clone();
                                let on_update_doc = on_update.clone();
                                let on_update_del = on_update.clone();
                                let contractor_id_doc = contractor_id.clone();
                                let contractor_id_del = contractor_id.clone();

                                let update_doc = move |updated_status: DocStatus| {
                                    set_docs.update(|d| {
                                        d.insert(key_clone.clone(), updated_status);
                                    });
                                    on_update_doc(Contractor {
                                        id: contractor_id_doc.clone(),
                                        name: name.get(),
                                        role: role.get(),
                                        docs: docs.get(),
                                    });
                                };

                                let delete_doc = move |_| {
                                    set_docs.update(|d| {
                                        d.remove(&key_for_delete);
                                    });
                                    on_update_del(Contractor {
                                        id: contractor_id_del.clone(),
                                        name: name.get(),
                                        role: role.get(),
                                        docs: docs.get(),
                                    });
                                };

                                view! {
                                    <DocEditor
                                        doc_key=key
                                        status=status
                                        on_update=update_doc
                                        on_delete=delete_doc
                                    />
                                }
                            }).collect_view()}
                        </div>
                    }
                })
            }}
        </div>
    }
}

/// 書類編集コンポーネント
#[component]
pub fn DocEditor<F, D>(
    doc_key: String,
    status: DocStatus,
    on_update: F,
    on_delete: D,
) -> impl IntoView
where
    F: Fn(DocStatus) + 'static + Clone,
    D: Fn(()) + 'static,
{
    let (doc_status, set_doc_status) = create_signal(status.status);
    let (file, set_file) = create_signal(status.file.clone().unwrap_or_default());
    let (url, set_url) = create_signal(status.url.clone().unwrap_or_default());
    let (valid_until, set_valid_until) = create_signal(status.valid_until.clone().unwrap_or_default());
    let (note, set_note) = create_signal(status.note.clone().unwrap_or_default());

    // 既存データを保持（編集時に消えないように）
    let original_valid_from = status.valid_from.clone();
    let original_check_result = status.check_result.clone();
    let original_last_checked = status.last_checked.clone();

    let label = doc_key.replace("_", " ").chars().skip_while(|c| c.is_numeric()).collect::<String>();
    let label = label.trim_start_matches('_').to_string();

    // 各イベント用にon_updateをクローン
    let on_update_1 = on_update.clone();
    let on_update_2 = on_update.clone();
    let on_update_3 = on_update.clone();
    let on_update_4 = on_update.clone();
    let on_update_5 = on_update;

    // 各ハンドラ用に既存値をクローン
    let (vf1, cr1, lc1) = (original_valid_from.clone(), original_check_result.clone(), original_last_checked.clone());
    let (vf2, cr2, lc2) = (original_valid_from.clone(), original_check_result.clone(), original_last_checked.clone());
    let (vf3, cr3, lc3) = (original_valid_from.clone(), original_check_result.clone(), original_last_checked.clone());
    let (vf4, cr4, lc4) = (original_valid_from.clone(), original_check_result.clone(), original_last_checked.clone());
    let (vf5, cr5, lc5) = (original_valid_from, original_check_result, original_last_checked);

    view! {
        <div class=format!("doc-editor {}", if doc_status.get() { "complete" } else { "incomplete" })>
            <div class="doc-editor-row">
                <label class="checkbox-label">
                    <input type="checkbox"
                        prop:checked=move || doc_status.get()
                        on:change=move |ev| {
                            set_doc_status.set(event_target_checked(&ev));
                            on_update_1(DocStatus {
                                status: doc_status.get(),
                                file: if file.get().is_empty() { None } else { Some(file.get()) },
                                url: if url.get().is_empty() { None } else { Some(url.get()) },
                                note: if note.get().is_empty() { None } else { Some(note.get()) },
                                valid_from: vf1.clone(),
                                valid_until: if valid_until.get().is_empty() { None } else { Some(valid_until.get()) },
                                check_result: cr1.clone(),
                                last_checked: lc1.clone(),
                            });
                        }
                    />
                    <span class="doc-label">{label}</span>
                </label>
                <button class="delete-btn small" on:click=move |_| on_delete(())>"✕"</button>
            </div>
            <div class="doc-editor-fields">
                <input type="text" placeholder="ファイル名"
                    prop:value=move || file.get()
                    on:input=move |ev| {
                        set_file.set(event_target_value(&ev));
                        on_update_2(DocStatus {
                            status: doc_status.get(),
                            file: if file.get().is_empty() { None } else { Some(file.get()) },
                            url: if url.get().is_empty() { None } else { Some(url.get()) },
                            note: if note.get().is_empty() { None } else { Some(note.get()) },
                            valid_from: vf2.clone(),
                            valid_until: if valid_until.get().is_empty() { None } else { Some(valid_until.get()) },
                            check_result: cr2.clone(),
                            last_checked: lc2.clone(),
                        });
                    }
                />
                <input type="text" placeholder="URL"
                    prop:value=move || url.get()
                    on:input=move |ev| {
                        set_url.set(event_target_value(&ev));
                        on_update_3(DocStatus {
                            status: doc_status.get(),
                            file: if file.get().is_empty() { None } else { Some(file.get()) },
                            url: if url.get().is_empty() { None } else { Some(url.get()) },
                            note: if note.get().is_empty() { None } else { Some(note.get()) },
                            valid_from: vf3.clone(),
                            valid_until: if valid_until.get().is_empty() { None } else { Some(valid_until.get()) },
                            check_result: cr3.clone(),
                            last_checked: lc3.clone(),
                        });
                    }
                />
                <input type="date" placeholder="有効期限"
                    prop:value=move || valid_until.get()
                    on:input=move |ev| {
                        set_valid_until.set(event_target_value(&ev));
                        on_update_4(DocStatus {
                            status: doc_status.get(),
                            file: if file.get().is_empty() { None } else { Some(file.get()) },
                            url: if url.get().is_empty() { None } else { Some(url.get()) },
                            note: if note.get().is_empty() { None } else { Some(note.get()) },
                            valid_from: vf4.clone(),
                            valid_until: if valid_until.get().is_empty() { None } else { Some(valid_until.get()) },
                            check_result: cr4.clone(),
                            last_checked: lc4.clone(),
                        });
                    }
                />
                <input type="text" placeholder="備考"
                    prop:value=move || note.get()
                    on:input=move |ev| {
                        set_note.set(event_target_value(&ev));
                        on_update_5(DocStatus {
                            status: doc_status.get(),
                            file: if file.get().is_empty() { None } else { Some(file.get()) },
                            url: if url.get().is_empty() { None } else { Some(url.get()) },
                            note: if note.get().is_empty() { None } else { Some(note.get()) },
                            valid_from: vf5.clone(),
                            valid_until: if valid_until.get().is_empty() { None } else { Some(valid_until.get()) },
                            check_result: cr5.clone(),
                            last_checked: lc5.clone(),
                        });
                    }
                />
            </div>
        </div>
    }
}
