//! Generates a per-build identifier (`BUILD_NUMBER`) that changes on every
//! deployment, and uses the same value to version the PWA service worker's
//! cache name.
//!
//! The deployment command sets `TYPING_BUILD` to a fresh value (e.g. a unix
//! timestamp) for the build; `cargo:rerun-if-env-changed` forces this script to
//! re-run whenever that value changes, and the value is baked into the binary.
//! That is what makes the number actually change per deployment even though
//! Cargo would otherwise reuse its cached artifacts.
//!
//! The same label is written to `public/build.json` and substituted into the
//! service worker template (`sw.template.js` -> `public/sw.js`) as the
//! `CACHE_NAME`. Because that name changes on every deploy, `sw.js`'s bytes
//! change too, which forces browsers to install a fresh service worker; its
//! `activate` handler then deletes the previous build's caches. Without a byte
//! change in `sw.js` the browser would keep reusing the old worker and its old
//! cache forever.
//!
//! When the variable is not set (local/dev builds) we fall back to the crate
//! version plus the short git revision, which is stable within a source tree.

fn main() {
    println!("cargo:rerun-if-env-changed=TYPING_BUILD");
    // Re-run if the build definition itself changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    // Re-generate the service worker whenever its template is edited.
    println!("cargo:rerun-if-changed=sw.template.js");

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

    // Generate the deployed service worker from the template, baking the same
    // per-build label into CACHE_NAME so that the PWA cache is versioned in
    // lockstep with the build number (see the module docs).
    let template_path = std::path::Path::new(&manifest_dir).join("sw.template.js");
    let template = std::fs::read_to_string(&template_path)
        .expect("failed to read sw.template.js (the service worker template)");
    let sw_source = template.replace("__CACHE_NAME__", &cache_name_from_label(&label));
    std::fs::write(assets_dir.join("sw.js"), sw_source)
        .expect("failed to write public/sw.js");
}

/// Derive a clean per-deploy cache name for the service worker from the same
/// build label that is baked into the binary. Cache names may be arbitrary
/// strings, but we keep them JS/URL-friendly: runs of invalid characters (e.g.
/// the space+paren in the dev fallback label) collapse to a single dash, with
/// no leading/trailing dashes.
fn cache_name_from_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len() + 8);
    for c in label.chars() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    format!("tippen-{out}")
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
