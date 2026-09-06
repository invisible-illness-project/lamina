//! Build script for `lamina_bridge`.
//!
//! Extracts the Lamina crate version from the workspace-root `Cargo.toml`
//! (two directories up from this crate) and exposes it to the binary as the
//! `LAMINA_VERSION` compile-time environment variable, so the `version` op can
//! report the exact Lamina version under test without hardcoding it.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let lamina_manifest = Path::new(&manifest_dir).join("../../Cargo.toml");
    println!("cargo:rerun-if-changed={}", lamina_manifest.display());

    let version = fs::read_to_string(&lamina_manifest)
        .ok()
        .and_then(|text| {
            let mut in_package = false;
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with('[') {
                    in_package = line == "[package]";
                    continue;
                }
                if in_package && line.starts_with("version") {
                    if let Some(v) = line.split('=').nth(1) {
                        return Some(v.trim().trim_matches('"').to_string());
                    }
                }
            }
            None
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=LAMINA_VERSION={version}");
}
