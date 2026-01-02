//! UIコンポーネントモジュール
//!
//! TODO: 以下のコンポーネントを個別ファイルに分割予定
//! - contractor_card.rs
//! - editors.rs (ProjectEditor, ContractorEditor, DocEditor)

pub mod tooltip;
pub mod project_view;

pub use tooltip::ContextMenu;
pub use project_view::{ProjectView, ProjectDocCard};
