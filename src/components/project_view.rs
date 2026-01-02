//! プロジェクト表示コンポーネント

use leptos::*;
use crate::models::{ProjectData, DocLink};
use super::ContractorCard;

/// プロジェクト全体の書類カード
#[component]
pub fn ProjectDocCard(label: &'static str, doc: Option<DocLink>) -> impl IntoView {
    let (has_doc, url, status) = match &doc {
        Some(d) => (true, d.url.clone(), d.status),
        None => (false, None, false),
    };

    view! {
        <div class=format!("project-doc-card {}", if status { "complete" } else if has_doc { "incomplete" } else { "empty" })>
            <span class="doc-icon">{
                if status { "✓" } else if has_doc { "○" } else { "−" }
            }</span>
            {if let Some(u) = url {
                view! {
                    <a class="doc-link" href=u target="_blank" rel="noopener">{label}</a>
                }.into_view()
            } else {
                view! {
                    <span class="doc-name">{label}</span>
                }.into_view()
            }}
        </div>
    }
}

/// プロジェクト詳細ビュー
#[component]
pub fn ProjectView(project: ProjectData) -> impl IntoView {
    let total_docs: usize = project.contractors.iter().map(|c| c.docs.len()).sum();
    let complete_docs: usize = project.contractors.iter()
        .flat_map(|c| c.docs.values())
        .filter(|d| d.status)
        .count();
    let progress = if total_docs > 0 { (complete_docs * 100) / total_docs } else { 0 };

    let project_docs = project.project_docs.clone();

    view! {
        <div class="project-view">
            <div class="project-header">
                <h3>{project.project_name.clone()}</h3>
                <div class="project-meta">
                    <span class="client">{project.client.clone()}</span>
                    <span class="period">{project.period.clone()}</span>
                </div>
            </div>

            <div class="progress-section">
                <div class="progress-bar">
                    <div class="progress-fill" style=format!("width: {}%", progress)></div>
                </div>
                <span class="progress-text">{complete_docs}"/" {total_docs} " (" {progress}"%)"</span>
            </div>

            // 全体書類セクション
            <div class="project-docs-section">
                <h4>"全体書類"</h4>
                <div class="project-docs-grid">
                    <ProjectDocCard
                        label="施工体系図"
                        doc=project_docs.sekou_taikeizu.clone()
                    />
                    <ProjectDocCard
                        label="施工体制台帳"
                        doc=project_docs.sekou_taisei_daicho.clone()
                    />
                    <ProjectDocCard
                        label="下請契約書"
                        doc=project_docs.shitauke_keiyaku.clone()
                    />
                </div>
            </div>

            // 各社書類セクション
            <div class="contractors-section">
                <h4>"各社書類"</h4>
                <div class="contractors-grid">
                    {project.contractors.into_iter().map(|c| view! {
                        <ContractorCard contractor=c />
                    }).collect_view()}
                </div>
            </div>

            // 下請施工体制セクション
            {(!project.contracts.is_empty()).then(|| view! {
                <div class="contracts-section">
                    <h4>"下請施工体制"</h4>
                    <div class="contracts-list">
                        {project.contracts.into_iter().map(|c| view! {
                            <div class="contract-item">
                                {if let Some(url) = c.url {
                                    view! {
                                        <a class="contract-link" href=url target="_blank" rel="noopener">{c.name}</a>
                                    }.into_view()
                                } else {
                                    view! {
                                        <span class="contract-name">{c.name}</span>
                                    }.into_view()
                                }}
                                {c.contractor.map(|contractor| view! {
                                    <span class="contract-contractor">{contractor}</span>
                                })}
                            </div>
                        }).collect_view()}
                    </div>
                </div>
            })}
        </div>
    }
}
