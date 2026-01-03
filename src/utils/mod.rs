//! ユーティリティモジュール

pub mod cache;
pub mod gas;
pub mod log_trace;

// 共通ヘルパー

/// Base64エンコード
pub fn encode_base64(data: &str) -> Option<String> {
    let window = web_sys::window()?;
    window.btoa(data).ok()
}

/// Base64デコード
pub fn decode_base64(data: &str) -> Option<String> {
    let window = web_sys::window()?;
    window.atob(data).ok()
}
