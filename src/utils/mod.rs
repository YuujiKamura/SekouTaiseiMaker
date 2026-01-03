//! ユーティリティモジュール

pub mod cache;
pub mod gas;
pub mod log_trace;

use base64::Engine;

// 共通ヘルパー

/// Base64エンコード（UTF-8安全）
/// btoa/atobは非ASCII文字（日本語）で壊れるため、base64クレートを使用
pub fn encode_base64(data: &str) -> Option<String> {
    Some(base64::engine::general_purpose::STANDARD.encode(data.as_bytes()))
}

/// Base64デコード（UTF-8安全）
pub fn decode_base64(data: &str) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}
