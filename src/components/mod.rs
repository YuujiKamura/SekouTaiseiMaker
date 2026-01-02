//! UIコンポーネントモジュール
//!
//! TODO: 以下のコンポーネントを個別ファイルに分割予定
//! - editors.rs (ProjectEditor, ContractorEditor, DocEditor)

pub mod contractor_card;
pub mod tooltip;
pub mod project_view;

pub use contractor_card::ContractorCard;
pub use tooltip::ContextMenu;
pub use project_view::{ProjectView, ProjectDocCard};
