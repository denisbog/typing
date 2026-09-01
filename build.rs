//! Generates a per-build identifier (`BUILD_NUMBER`) that changes on every
//! deployment.
//!
//! The deployment command sets `TYPING_BUILD` to a fresh value (e.g. a unix
//! timestamp) for the build; `cargo:rerun-if-env-changed` forces this script to
//! re-run whenever that value changes, and the value is baked into the binary.
//! That is what makes the number actually change per deployment even though
//! Cargo would otherwise reuse its cached artifacts.
//!
//! When the variable is not set (local/dev builds) we fall back to the crate
//! version plus the short git revision, which is stable within a source tree.

fn main() {
    println!("cargo:rerun-if-env-changed=TYPING_BUILD");
    // Re-run if the build definition itself changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let label = std::env::var("TYPING_BUILD")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|v| format!("build-{v}"))
        .unwrap_or_else(|| {
            format!("{} ({})", env!("CARGO_PKG_VERSION"), short_git_revision())
        });

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let dest = std::path::Path::new(&out_dir).join("build_info.rs");
    std::fs::write(
        &dest,
        format!("pub const BUILD_NUMBER: &str = {label:?};\n"),
    )
    .expect("failed to write build_info.rs");

    // Also expose the build number as a static JSON file in the assets dir so
    // it can be read from a deployed static file server (or by any client
    // without loading the wasm bundle). cargo-leptos asset-syncs `public/`
    // into `target/site`, which is what gets deployed, so this lands there and
    // is served at `/build.json`.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let assets_dir = std::path::Path::new(&manifest_dir).join("public");
    if !assets_dir.exists() {
        std::fs::create_dir_all(&assets_dir).expect("failed to create public dir");
    }
    let build_info_file = assets_dir.join("build.json");
    std::fs::write(
        &build_info_file,
        format!("{{\"build\": {label:?}}}\n"),
    )
    .expect("failed to write build.json");
}

fn short_git_revision() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string())
}
