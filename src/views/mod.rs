//! ビューモジュール
//!
//! TODO: 以下のビューを個別ファイルに分割予定
//! - dashboard.rs
//! - spreadsheet_viewer.rs
//! - ocr_viewer.rs
//! - settings.rs (APIキー設定画面)

pub mod check_panel;
pub mod pdf_viewer;

pub use check_panel::{CheckResultPanel, CheckResultsPanel};
pub use pdf_viewer::{PdfViewer, ViewerCheckResultPanel};
