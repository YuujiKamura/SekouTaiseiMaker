use std::fs;
use std::process::Command;
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

    // health-report.htmlを生成（Windows/Linux両対応）
    // distディレクトリが存在する場合のみ実行（Trunkのビルド後）
    if fs::metadata("dist").is_ok() {
        let exe_path = if cfg!(windows) {
            "tools\\codebase-health\\target\\release\\codebase-health.exe"
        } else {
            "tools/codebase-health/target/release/codebase-health"
        };

        if fs::metadata(exe_path).is_ok() {
            let output = Command::new(exe_path)
                .args(&["analyze", "--format", "html", "--output", "dist/health-report.html"])
                .output();

            if let Ok(result) = output {
                if !result.status.success() {
                    println!("cargo:warning=Failed to generate health-report.html: {:?}", result.stderr);
                }
            }
        }
    }
}



