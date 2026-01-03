//! データ構造体モジュール

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================
// 施工体制ダッシュボード用データ構造
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectData {
    pub project_name: String,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub period: String,
    #[serde(default)]
    pub period_start: Option<String>,
    #[serde(default)]
    pub period_end: Option<String>,
    #[serde(default)]
    pub site_agent: Option<String>,
    #[serde(default)]
    pub chief_engineer: Option<String>,
    #[serde(default)]
    pub project_docs: ProjectDocs,
    pub contractors: Vec<Contractor>,
    #[serde(default)]
    pub contracts: Vec<Contract>,
}

/// 全体書類（施工体系図、施工体制台帳、下請契約書）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectDocs {
    #[serde(default)]
    pub sekou_taikeizu: Option<DocLink>,
    #[serde(default)]
    pub sekou_taisei_daicho: Option<DocLink>,
    #[serde(default)]
    pub shitauke_keiyaku: Option<DocLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocLink {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contractor {
    pub id: String,
    pub name: String,
    pub role: String,
    pub docs: HashMap<String, DocStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocStatus {
    pub status: bool,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub valid_from: Option<String>,
    #[serde(default)]
    pub valid_until: Option<String>,
    #[serde(default)]
    pub check_result: Option<CheckResultData>,
    #[serde(default)]
    pub last_checked: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contract {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub contractor: Option<String>,
}

// ============================================
// AIチェック結果
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckResultData {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub items: Vec<CheckItem>,
    #[serde(default)]
    pub missing_fields: Vec<CheckMissingField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    #[serde(rename = "type")]
    pub item_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckMissingField {
    pub field: String,
    pub location: String,
}

// ============================================
// UI状態
// ============================================

/// チェック結果ツールチップ状態（ホバーで表示）
#[derive(Clone, Default)]
pub struct CheckResultTooltipState {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub contractor_name: String,
    pub doc_key: String,
    pub doc_label: String,
    pub check_result: Option<CheckResultData>,
    pub last_checked: Option<String>,
    pub hover_timer_id: Option<i32>,
}

/// コンテキストメニュー状態（右クリック/ロングプレスで表示）
#[derive(Clone, Default)]
pub struct ContextMenuState {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub contractor_name: String,
    pub contractor_id: String,
    pub doc_key: String,
    pub doc_label: String,
    pub url: Option<String>,
    pub has_check_result: bool,
}

// ============================================
// ビューモード
// ============================================

#[derive(Clone, PartialEq)]
pub enum ViewMode {
    Dashboard,
    OcrViewer,
    ApiKeySetup,
    AiChecker {
        contractor: String,
        doc_type: String,
        file_id: String,
        doc_key: String,
        contractor_id: String,
    },
    PdfViewer {
        contractor: String,
        doc_type: String,
        url: String,
        doc_key: String,
        contractor_id: String,
    },
    SpreadsheetViewer {
        contractor: String,
        doc_type: String,
        url: String,
        doc_key: String,
        contractor_id: String,
    },
    PdfEditor {
        contractor: String,
        doc_type: String,
        original_url: String,
    },
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Dashboard
    }
}

// ============================================
// ファイルタイプ
// ============================================

#[derive(Clone, PartialEq, Debug)]
pub enum DocFileType {
    Pdf,
    GoogleSpreadsheet,
    Excel,
    GoogleDoc,
    Image,
    Unknown,
}

/// ファイルタイプ判定
///
/// ## 変更履歴
/// - 2026-01-02: rtpof=trueでExcelファイルを判定するロジック追加
/// - 2026-01-02: drive.google.com/file URLはMIMEタイプを付与して判定するように変更
pub fn detect_file_type(url: &str) -> DocFileType {
    let url_lower = url.to_lowercase();

    // Google Spreadsheetとして開かれているExcelファイル（rtpof=true）
    if url_lower.contains("docs.google.com/spreadsheets") && url_lower.contains("rtpof=true") {
        DocFileType::Excel
    } else if url_lower.contains("docs.google.com/spreadsheets") {
        DocFileType::GoogleSpreadsheet
    } else if url_lower.contains("docs.google.com/document") {
        DocFileType::GoogleDoc
    } else if url_lower.contains("drive.google.com/file") {
        // Google DriveのファイルURLはMIMEタイプで判定
        // URL末尾やクエリパラメータにMIMEタイプヒントがある場合
        if url_lower.contains("mime=application/pdf") || url_lower.contains("type=pdf") {
            DocFileType::Pdf
        } else if url_lower.contains("mime=application/vnd.openxmlformats")
            || url_lower.contains("mime=application/vnd.ms-excel")
            || url_lower.contains("type=xlsx")
            || url_lower.contains("type=xls")
        {
            DocFileType::Excel
        } else if url_lower.contains("mime=image/") || url_lower.contains("type=image") {
            DocFileType::Image
        } else {
            // MIMEタイプが不明な場合、URLにヒントがあれば使う
            // ファイル名が含まれている場合（例: /d/xxx/view?filename=xxx.pdf）
            if url_lower.contains(".pdf") {
                DocFileType::Pdf
            } else if url_lower.contains(".xlsx") || url_lower.contains(".xls") {
                DocFileType::Excel
            } else if url_lower.contains(".png")
                || url_lower.contains(".jpg")
                || url_lower.contains(".jpeg")
            {
                DocFileType::Image
            } else {
                // 判別不能な場合はPDFとして試行（従来の動作）
                // ただし失敗した場合はエラーメッセージを表示
                DocFileType::Pdf
            }
        }
    } else if url_lower.ends_with(".pdf") {
        DocFileType::Pdf
    } else if url_lower.ends_with(".xlsx") || url_lower.ends_with(".xls") {
        DocFileType::Excel
    } else if url_lower.ends_with(".png")
        || url_lower.ends_with(".jpg")
        || url_lower.ends_with(".jpeg")
    {
        DocFileType::Image
    } else {
        DocFileType::Unknown
    }
}
