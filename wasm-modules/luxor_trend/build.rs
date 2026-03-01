//! Build script for luxor_trend WASM module
//!
//! Ensures proper WASM target configuration and provides build-time metadata.

use std::env;

fn main() {
    // Check if we're building for wasm32 target
    let target = env::var("TARGET").unwrap_or_default();

    if !target.contains("wasm32") {
        // Not a WASM target, print a warning but allow the build to continue
        // for development/testing purposes
        println!(
            "cargo:warning=Building luxor_trend for non-WASM target: {}",
            target
        );
    } else {
        println!("cargo:rustc-cfg=target_wasm32");
    }

    // Re-run build script if Cargo.toml changes
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");

    // Set version info
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    println!("cargo:rustc-env=LUXOR_TREND_VERSION={}", version);
}
