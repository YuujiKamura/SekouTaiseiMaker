use std::fs;
use std::time::UNIX_EPOCH;

fn main() {
    // gas/SekouTaiseiSync.gs の更新日時を取得
    if let Ok(metadata) = fs::metadata("gas/SekouTaiseiSync.gs") {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                let timestamp = duration.as_secs();
                println!("cargo:warning=GAS_SCRIPT_MODIFIED set to {}", timestamp);
                println!("cargo:rustc-env=GAS_SCRIPT_MODIFIED={}", timestamp);
            }
        }
    } else {
        println!("cargo:warning=Could not read gas/SekouTaiseiSync.gs");
    }

    // ファイル変更時に再ビルド
    println!("cargo:rerun-if-changed=gas/SekouTaiseiSync.gs");
}

