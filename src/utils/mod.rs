//! ユーティリティモジュール

pub mod cache;
pub mod gas;

use base64::Engine;

// 共通ヘルパー

/// Base64エンコード
pub fn encode_base64(data: &str) -> Option<String> {
    Some(base64::engine::general_purpose::STANDARD.encode(data.as_bytes()))
}

/// Base64デコード
pub fn decode_base64(data: &str) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}
