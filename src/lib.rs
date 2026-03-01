//! IronClaw — Secure WASM Sandbox Engine for QuantTrader
//! (ARCHITECTURE.md v1.0 — landlock/seccomp + bubblewrap + wasmtime)
//! Host runtime ONLY — never builds for wasm32 target

// Prevent accidental wasm builds - ironclaw is a host runtime that requires native OS
#[cfg(target_arch = "wasm32")]
compile_error!(
    "ironclaw is a host runtime and cannot be built for wasm32 targets. \
     Build for native target (x86_64-unknown-linux-gnu, aarch64-apple-darwin, etc.)"
);

pub mod sandbox;
pub mod strategy_runner;

pub use sandbox::SandboxConfig;
pub use strategy_runner::{
    Action, BacktestResult, MarketData, ResourceUsage, Signal, StrategyRuntime, Trade,
    WasmtimeRunner,
};
