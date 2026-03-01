//! Build script for IronClaw
//!
//! This script validates the build target and embeds registry manifests.
//! IronClaw is a host runtime and cannot be built for WASM targets.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // ── Validate build target (host runtime only) ─────────────────────────
    let target = env::var("TARGET").expect("TARGET env var should be set by Cargo");

    if target.starts_with("wasm32-") {
        eprintln!(
            "\n╔══════════════════════════════════════════════════════════════════════════════╗"
        );
        eprintln!(
            "║                            BUILD ERROR                                       ║"
        );
        eprintln!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        eprintln!(
            "║  IronClaw is a HOST RUNTIME that runs on native operating systems.           ║"
        );
        eprintln!(
            "║                                                                              ║"
        );
        eprintln!(
            "║  It CANNOT be built for WASM targets like:                                   ║"
        );
        eprintln!(
            "║    - wasm32-unknown-unknown                                                  ║"
        );
        eprintln!(
            "║    - wasm32-wasip1                                                           ║"
        );
        eprintln!(
            "║    - wasm32-wasip2                                                           ║"
        );
        eprintln!(
            "║                                                                              ║"
        );
        eprintln!(
            "║  The host runtime:                                                           ║"
        );
        eprintln!(
            "║    - Loads and executes WASM strategy modules                                ║"
        );
        eprintln!(
            "║    - Uses tokio, wasmtime, landlock, seccomp (native OS APIs)                ║"
        );
        eprintln!(
            "║    - Requires an OS with errno support (Linux, macOS, Windows)               ║"
        );
        eprintln!(
            "║                                                                              ║"
        );
        eprintln!(
            "║  Correct build command:                                                      ║"
        );
        eprintln!(
            "║    cargo build -p ironclaw                                                   ║"
        );
        eprintln!(
            "║                                                                              ║"
        );
        eprintln!(
            "║  If you need WASM strategy modules, build the channels separately:           ║"
        );
        eprintln!(
            "║    ./channels-src/telegram/build.sh                                          ║"
        );
        eprintln!(
            "╚══════════════════════════════════════════════════════════════════════════════╝\n"
        );

        panic!("IronClaw cannot be built for WASM target: {}", target);
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = PathBuf::from(&manifest_dir);

    // ── Embed registry manifests ────────────────────────────────────────
    embed_registry_catalog(&root);

    // ── Build Telegram channel WASM ─────────────────────────────────────
    let channel_dir = root.join("channels-src/telegram");
    let wasm_out = channel_dir.join("telegram.wasm");

    // Rerun when channel source or build script changes
    println!("cargo:rerun-if-changed=channels-src/telegram/src");
    println!("cargo:rerun-if-changed=channels-src/telegram/Cargo.toml");
    println!("cargo:rerun-if-changed=wit/channel.wit");

    if !channel_dir.is_dir() {
        return;
    }

    // Build WASM module
    let status = match Command::new("cargo")
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip2",
            "--manifest-path",
            channel_dir.join("Cargo.toml").to_str().unwrap(),
        ])
        .current_dir(&root)
        .status()
    {
        Ok(s) => s,
        Err(_) => {
            eprintln!(
                "cargo:warning=Telegram channel build failed. Run: ./channels-src/telegram/build.sh"
            );
            return;
        }
    };

    if !status.success() {
        eprintln!(
            "cargo:warning=Telegram channel build failed. Run: ./channels-src/telegram/build.sh"
        );
        return;
    }

    let raw_wasm = channel_dir.join("target/wasm32-wasip2/release/telegram_channel.wasm");
    if !raw_wasm.exists() {
        eprintln!(
            "cargo:warning=Telegram WASM output not found at {:?}",
            raw_wasm
        );
        return;
    }

    // Convert to component and strip (wasm-tools)
    let component_ok = Command::new("wasm-tools")
        .args([
            "component",
            "new",
            raw_wasm.to_str().unwrap(),
            "-o",
            wasm_out.to_str().unwrap(),
        ])
        .current_dir(&root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !component_ok {
        // Fallback: copy raw module if wasm-tools unavailable
        if std::fs::copy(&raw_wasm, &wasm_out).is_err() {
            eprintln!("cargo:warning=wasm-tools not found. Run: cargo install wasm-tools");
        }
    } else {
        // Strip debug info (use temp file to avoid clobbering)
        let stripped = wasm_out.with_extension("wasm.stripped");
        let strip_ok = Command::new("wasm-tools")
            .args([
                "strip",
                wasm_out.to_str().unwrap(),
                "-o",
                stripped.to_str().unwrap(),
            ])
            .current_dir(&root)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if strip_ok {
            let _ = std::fs::rename(&stripped, &wasm_out);
        }
    }
}

/// Collect all registry manifests into a single JSON blob at compile time.
fn embed_registry_catalog(root: &Path) {
    use std::fs;

    let registry_dir = root.join("registry");

    println!("cargo:rerun-if-changed=registry/_bundles.json");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = out_dir.join("embedded_catalog.json");

    if !registry_dir.is_dir() {
        fs::write(
            &out_path,
            r#"{"tools":[],"channels":[],"bundles":{"bundles":{}}}"#,
        )
        .unwrap();
        return;
    }

    let mut tools = Vec::new();
    let mut channels = Vec::new();

    let tools_dir = registry_dir.join("tools");
    if tools_dir.is_dir() {
        collect_json_files(&tools_dir, &mut tools);
    }

    let channels_dir = registry_dir.join("channels");
    if channels_dir.is_dir() {
        collect_json_files(&channels_dir, &mut channels);
    }

    let bundles_path = registry_dir.join("_bundles.json");
    let bundles_raw = if bundles_path.is_file() {
        fs::read_to_string(&bundles_path).unwrap_or_else(|_| r#"{"bundles":{}}"#.to_string())
    } else {
        r#"{"bundles":{}}"#.to_string()
    };

    let catalog = format!(
        r#"{{"tools":[{}],"channels":[{}],"bundles":{}}}"#,
        tools.join(","),
        channels.join(","),
        bundles_raw,
    );

    fs::write(&out_path, catalog).unwrap();
}

/// Read all .json files from a directory and push their raw contents into `out`.
fn collect_json_files(dir: &Path, out: &mut Vec<String>) {
    use std::fs;

    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file() && e.path().extension().and_then(|x| x.to_str()) == Some("json")
        })
        .collect();

    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        println!("cargo:rerun-if-changed={}", entry.path().display());
        if let Ok(content) = fs::read_to_string(entry.path()) {
            out.push(content);
        }
    }
}
