//! LocalStorageキャッシュ管理

use crate::models::ProjectData;

const CACHE_KEY: &str = "sekou_taisei_cache";

/// プロジェクトデータをキャッシュに保存
pub fn save_to_cache(project: &ProjectData) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(project) {
                let _ = storage.set_item(CACHE_KEY, &json);
            }
        }
    }
}

/// キャッシュからプロジェクトデータを読み込み
pub fn load_from_cache() -> Option<ProjectData> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let json = storage.get_item(CACHE_KEY).ok()??;
    serde_json::from_str(&json).ok()
}

/// キャッシュをクリア
pub fn clear_cache() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(CACHE_KEY);
        }
    }
}
