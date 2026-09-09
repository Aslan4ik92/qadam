//! Make sure the frontend bundle directory exists so `include_dir!` compiles
//! even when the UI has not been built yet (a placeholder page is embedded).

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dist = manifest.join("../../apps/desktop/dist");
    println!("cargo:rerun-if-changed={}", dist.display());
    if !dist.join("index.html").exists() {
        let _ = std::fs::create_dir_all(&dist);
        let _ = std::fs::write(
            dist.join("index.html"),
            "<!doctype html><meta charset=utf-8><title>QIDIR</title><p>Интерфейс QIDIR не собран: выполните <code>npm run build</code> в <code>apps/desktop</code>.</p>",
        );
    }
}
