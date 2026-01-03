//! 時系列トレースログシステム
//! すべての操作とイベントを自動記録し、後から確認できるようにする

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_LOG_ENTRIES: usize = 1000;
const STORAGE_KEY: &str = "sekou_taisei_log_trace";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String, // "info", "warn", "error", "debug"
    pub category: String, // "ai-check", "gas-sync", "ui-action", etc.
    pub message: String,
    pub data: Option<serde_json::Value>,
}

pub struct LogTrace {
    logs: VecDeque<LogEntry>,
}

impl LogTrace {
    pub fn new() -> Self {
        let mut trace = LogTrace {
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
        };
        trace.load_from_storage();
        trace
    }

    pub fn log(&mut self, level: &str, category: &str, message: &str, data: Option<serde_json::Value>) {
        let timestamp = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
        
        let entry = LogEntry {
            timestamp,
            level: level.to_string(),
            category: category.to_string(),
            message: message.to_string(),
            data,
        };

        // コンソールにも出力
        match level {
            "error" => web_sys::console::error_1(&format!("[{}] {}", category, message).into()),
            "warn" => web_sys::console::warn_1(&format!("[{}] {}", category, message).into()),
            _ => web_sys::console::log_1(&format!("[{}] {}", category, message).into()),
        }

        // ログを追加
        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);

        // 自動保存（非同期で実行）
        self.save_to_storage_async();
    }

    pub fn info(&mut self, category: &str, message: &str) {
        self.log("info", category, message, None);
    }

    pub fn info_with_data(&mut self, category: &str, message: &str, data: serde_json::Value) {
        self.log("info", category, message, Some(data));
    }

    pub fn warn(&mut self, category: &str, message: &str) {
        self.log("warn", category, message, None);
    }

    pub fn error(&mut self, category: &str, message: &str) {
        self.log("error", category, message, None);
    }

    pub fn error_with_data(&mut self, category: &str, message: &str, data: serde_json::Value) {
        self.log("error", category, message, Some(data));
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.iter().cloned().collect()
    }

    pub fn get_logs_json(&self) -> String {
        let logs: Vec<&LogEntry> = self.logs.iter().collect();
        serde_json::to_string_pretty(&logs).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn clear(&mut self) {
        self.logs.clear();
        self.save_to_storage();
    }

    fn load_from_storage(&mut self) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(json_str)) = storage.get_item(STORAGE_KEY) {
                    if let Ok(logs) = serde_json::from_str::<Vec<LogEntry>>(&json_str) {
                        self.logs = logs.into_iter().collect();
                        return;
                    }
                }
            }
        }
    }

    fn save_to_storage(&self) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let json_str = serde_json::to_string(&self.get_logs()).unwrap_or_else(|_| "[]".to_string());
                let _ = storage.set_item(STORAGE_KEY, &json_str);
            }
        }
    }

    fn save_to_storage_async(&self) {
        // 非同期で保存（パフォーマンスを考慮）
        let logs = self.get_logs();
        let json_str = serde_json::to_string(&logs).unwrap_or_else(|_| "[]".to_string());
        
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(STORAGE_KEY, &json_str);
            }
        }
    }

    pub fn download_logs(&self) {
        let json_str = self.get_logs_json();
        let timestamp = js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default();
        let filename = format!("log_trace_{}.json", timestamp.replace(":", "-").replace(".", "-"));

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let blob_parts = js_sys::Array::new();
                blob_parts.push(&JsValue::from_str(&json_str));
                
                let options = web_sys::BlobPropertyBag::new();
                options.set_type("application/json");

                if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                        if let Ok(a) = document.create_element("a") {
                            let _ = a.set_attribute("href", &url);
                            let _ = a.set_attribute("download", &filename);
                            if let Some(element) = a.dyn_ref::<web_sys::HtmlElement>() {
                                let _ = element.click();
                            }
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    }
}

// グローバルなログトレースインスタンス
thread_local! {
    static LOG_TRACE: std::cell::RefCell<LogTrace> = std::cell::RefCell::new(LogTrace::new());
}

pub fn log_info(category: &str, message: &str) {
    LOG_TRACE.with(|trace| {
        trace.borrow_mut().info(category, message);
    });
}

pub fn log_info_with_data(category: &str, message: &str, data: serde_json::Value) {
    LOG_TRACE.with(|trace| {
        trace.borrow_mut().info_with_data(category, message, data);
    });
}

pub fn log_warn(category: &str, message: &str) {
    LOG_TRACE.with(|trace| {
        trace.borrow_mut().warn(category, message);
    });
}

pub fn log_error(category: &str, message: &str) {
    LOG_TRACE.with(|trace| {
        trace.borrow_mut().error(category, message);
    });
}

pub fn log_error_with_data(category: &str, message: &str, data: serde_json::Value) {
    LOG_TRACE.with(|trace| {
        trace.borrow_mut().error_with_data(category, message, data);
    });
}

pub fn download_logs() {
    LOG_TRACE.with(|trace| {
        trace.borrow().download_logs();
    });
}

pub fn clear_logs() {
    LOG_TRACE.with(|trace| {
        trace.borrow_mut().clear();
    });
}

pub fn get_logs_json() -> String {
    LOG_TRACE.with(|trace| {
        trace.borrow().get_logs_json()
    })
}

pub async fn copy_logs_to_clipboard_async() -> Result<(), String> {
    let json_str = get_logs_json();
    
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();
        let clipboard = navigator.clipboard();
        
        let promise = clipboard.write_text(&json_str);
        let result = wasm_bindgen_futures::JsFuture::from(promise).await;
        
        match result {
            Ok(_) => {
                log_info("log-trace", "ログをクリップボードにコピーしました");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("クリップボードへのコピー失敗: {:?}", e);
                log_error("log-trace", &error_msg);
                Err(error_msg)
            }
        }
    } else {
        Err("windowが利用できません".to_string())
    }
}

