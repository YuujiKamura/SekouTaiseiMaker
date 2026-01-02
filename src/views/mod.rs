//! ビューモジュール
//!
//! TODO: 以下のビューを個別ファイルに分割予定
//! - dashboard.rs
//! - settings.rs (APIキー設定画面)

pub mod check_panel;
pub mod pdf_viewer;
pub mod ocr_viewer;
pub mod spreadsheet_viewer;

pub use check_panel::CheckResultsPanel;
pub use pdf_viewer::PdfViewer;
pub use spreadsheet_viewer::SpreadsheetViewer;
