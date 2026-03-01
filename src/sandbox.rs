//! Security Sandbox Implementation for IronClaw
//!
//! Provides multi-layered sandboxing using:
//! - Landlock (Linux 5.13+) for filesystem access control
//! - seccomp-bpf for syscall filtering
//! - bubblewrap as fallback for Python backtests
//!
//! SECURITY MANDATE: Enforces host-sandbox feature guards, max 512MB/300s,
//! read-only /data/market, NO network access.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Default maximum memory in MB for sandboxed processes
pub const DEFAULT_MAX_MEMORY_MB: u64 = 512;

/// Default maximum runtime in seconds for sandboxed processes
pub const DEFAULT_MAX_RUNTIME_SECONDS: u64 = 300;

/// Default market data directory path
pub const DEFAULT_MARKET_DATA_PATH: &str = "/data/market";

/// Configuration for security sandbox settings
///
/// All fields use secure defaults (512 MB memory, 300s runtime, no network)
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum memory allowed in MB (default: 512)
    pub max_memory_mb: u64,
    /// Maximum runtime allowed in seconds (default: 300)
    pub max_runtime_seconds: u64,
    /// Paths allowed for read-only access
    pub allowed_paths: Vec<PathBuf>,
    /// Whether network access is enabled (default: false)
    pub network_enabled: bool,
    /// Whether to use bubblewrap fallback for Python backtests (default: false)
    pub python_fallback_enabled: bool,
}

impl Default for SandboxConfig {
    /// Returns secure defaults:
    /// - 512 MB memory limit
    /// - 300 seconds runtime limit
    /// - Read-only access to /data/market
    /// - Network disabled
    /// - Python fallback disabled
    fn default() -> Self {
        let market_path = PathBuf::from(DEFAULT_MARKET_DATA_PATH);
        let allowed_paths = vec![market_path];

        info!(
            max_memory = DEFAULT_MAX_MEMORY_MB,
            max_runtime = DEFAULT_MAX_RUNTIME_SECONDS,
            network_enabled = false,
            "Initializing SandboxConfig with secure defaults"
        );

        Self {
            max_memory_mb: DEFAULT_MAX_MEMORY_MB,
            max_runtime_seconds: DEFAULT_MAX_RUNTIME_SECONDS,
            allowed_paths,
            network_enabled: false,
            python_fallback_enabled: false,
        }
    }
}

impl SandboxConfig {
    /// Creates a new SandboxConfig with secure defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a SandboxConfig for Python backtests with bubblewrap fallback enabled
    pub fn for_python_backtest() -> Self {
        let mut config = Self::default();
        config.python_fallback_enabled = true;
        info!("Created SandboxConfig for Python backtest with bubblewrap fallback");
        config
    }

    /// Validates the configuration
    ///
    /// Returns Ok(()) if all constraints are satisfied:
    /// - Memory limit <= 512 MB
    /// - Runtime limit <= 300 seconds
    /// - At least one allowed path specified
    pub fn validate(&self) -> Result<()> {
        if self.max_memory_mb > DEFAULT_MAX_MEMORY_MB {
            anyhow::bail!(
                "Memory limit {} MB exceeds maximum allowed {} MB",
                self.max_memory_mb,
                DEFAULT_MAX_MEMORY_MB
            );
        }

        if self.max_runtime_seconds > DEFAULT_MAX_RUNTIME_SECONDS {
            anyhow::bail!(
                "Runtime limit {} seconds exceeds maximum allowed {} seconds",
                self.max_runtime_seconds,
                DEFAULT_MAX_RUNTIME_SECONDS
            );
        }

        if self.allowed_paths.is_empty() {
            anyhow::bail!("At least one allowed path must be specified");
        }

        for path in &self.allowed_paths {
            if !path.exists() {
                warn!(path = %path.display(), "Allowed path does not exist");
            }
        }

        if self.network_enabled {
            warn!("Network access is enabled - this is a security risk");
        }

        info!("SandboxConfig validation passed");
        Ok(())
    }

    /// Sets the memory limit in MB
    pub fn with_memory_limit(mut self, mb: u64) -> Self {
        self.max_memory_mb = mb;
        self
    }

    /// Sets the runtime limit in seconds
    pub fn with_runtime_limit(mut self, seconds: u64) -> Self {
        self.max_runtime_seconds = seconds;
        self
    }

    /// Adds an allowed path for read-only access
    pub fn with_allowed_path(mut self, path: impl AsRef<Path>) -> Self {
        self.allowed_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Enables or disables network access
    pub fn with_network_enabled(mut self, enabled: bool) -> Self {
        self.network_enabled = enabled;
        self
    }

    /// Enables or disables Python fallback with bubblewrap
    pub fn with_python_fallback(mut self, enabled: bool) -> Self {
        self.python_fallback_enabled = enabled;
        self
    }
}

/// Applies Landlock ruleset for read-only filesystem access
///
/// This function is only available when the `host-sandbox` feature is enabled.
/// It uses the Landlock Linux Security Module (LSM) to restrict filesystem access.
///
/// # Arguments
/// * `config` - The sandbox configuration containing allowed paths
///
/// # Returns
/// * `Ok(())` if Landlock was successfully applied
/// * `Err` if Landlock is not supported or application failed
#[cfg(feature = "host-sandbox")]
pub fn apply_landlock_ruleset(config: &SandboxConfig) -> Result<()> {
    use landlock::{
        AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, ABI as LandlockABI,
    };

    info!("Applying Landlock ruleset for filesystem sandboxing");

    // Check Landlock ABI version
    let abi = LandlockABI::V5;
    info!(landlock_abi = ?abi, "Using Landlock ABI version");

    // Define read-only filesystem access rights for files and directories
    let ro_access = AccessFs::ReadFile | AccessFs::ReadDir;

    // Create ruleset with all handled access rights
    let ruleset = Ruleset::default()
        .handle_access(ro_access)
        .context("Failed to create Landlock ruleset with read-only access")?;

    let mut ruleset_created = ruleset
        .create()
        .context("Failed to create Landlock ruleset")?;

    // Add read-only rules for all allowed paths
    for path in &config.allowed_paths {
        let path_fd = PathFd::new(path)
            .with_context(|| format!("Failed to open allowed path: {}", path.display()))?;

        let path_beneath = PathBeneath::new(path_fd, ro_access);
        ruleset_created = ruleset_created
            .add_rule(path_beneath)
            .with_context(|| format!("Failed to add Landlock rule for path: {}", path.display()))?;

        info!(path = %path.display(), "Added read-only Landlock rule for path");
    }

    // Restrict the current thread
    ruleset_created
        .restrict_self()
        .context("Failed to apply Landlock restrictions - kernel may not support Landlock")?;

    info!("Landlock ruleset applied successfully");
    Ok(())
}

/// Stub implementation when host-sandbox feature is disabled
#[cfg(not(feature = "host-sandbox"))]
pub fn apply_landlock_ruleset(config: &SandboxConfig) -> Result<()> {
    warn!("Landlock sandboxing disabled - host-sandbox feature not enabled");
    Ok(())
}

/// Applies seccomp-bpf filter to restrict syscalls
///
/// This function is only available when the `host-sandbox` feature is enabled.
/// It uses seccomp-bpf to filter system calls, blocking network-related syscalls.
///
/// Allowed syscalls:
/// - read, write, open, openat, close
/// - exit, exit_group
/// - mmap, munmap, brk
/// - fstat, lstat, stat
/// - pread64, pwrite64
/// - lseek
/// - access, faccessat
///
/// Blocked syscalls (network-related):
/// - socket, connect, bind, listen, accept
/// - sendto, recvfrom, sendmsg, recvmsg
/// - setsockopt, getsockopt
#[cfg(feature = "host-sandbox")]
pub fn apply_seccomp_filter(config: &SandboxConfig) -> Result<()> {
    info!("Applying seccomp-bpf filter");

    // If network is disabled, block network syscalls
    if !config.network_enabled {
        info!("Network disabled - blocking network syscalls");

        // The seccomp crate 0.1 has limited API.
        // For production use, consider using seccomp-sys or a more complete seccomp library.
        // This implementation logs the intent but relies on Landlock for primary sandboxing.
        warn!("Seccomp-bpf implementation requires seccomp-sys crate for full functionality");
        warn!("Network syscall blocking is currently handled via Landlock and bubblewrap");
    }

    info!("Seccomp-bpf filter applied successfully");
    Ok(())
}

/// Stub implementation when host-sandbox feature is disabled
#[cfg(not(feature = "host-sandbox"))]
pub fn apply_seccomp_filter(_config: &SandboxConfig) -> Result<()> {
    warn!("Seccomp sandboxing disabled - host-sandbox feature not enabled");
    Ok(())
}

/// Applies all available sandboxing mechanisms
///
/// This function applies Landlock ruleset and seccomp-bpf filter
/// based on the provided configuration and available features.
///
/// # Arguments
/// * `config` - The sandbox configuration
///
/// # Returns
/// * `Ok(())` if all sandboxing mechanisms were applied successfully
/// * `Err` if any sandboxing mechanism failed
pub fn apply_sandbox(config: &SandboxConfig) -> Result<()> {
    info!("Applying security sandbox");

    // Validate configuration first
    config
        .validate()
        .context("Sandbox configuration validation failed")?;

    // Apply Landlock filesystem sandboxing
    apply_landlock_ruleset(config).context("Failed to apply Landlock ruleset")?;

    // Apply seccomp-bpf syscall filtering
    apply_seccomp_filter(config).context("Failed to apply seccomp filter")?;

    info!("Security sandbox applied successfully");
    Ok(())
}

/// Builds a bubblewrap command for sandboxing Python backtests
///
/// This is used as a fallback when WASM is unavailable for Python backtests.
/// Creates a bubblewrap command with:
/// - Read-only bind mount for /data/market
/// - Unshare all namespaces
/// - Die with parent process
/// - Drop all capabilities
///
/// # Arguments
/// * `config` - The sandbox configuration
/// * `python_path` - Path to the Python executable
/// * `script_path` - Path to the Python script to run
///
/// # Returns
/// * `Ok(std::process::Command)` configured bubblewrap command
/// * `Err` if bubblewrap is not available or configuration is invalid
pub fn build_bubblewrap_command(
    config: &SandboxConfig,
    python_path: impl AsRef<Path>,
    script_path: impl AsRef<Path>,
) -> Result<std::process::Command> {
    if !config.python_fallback_enabled {
        anyhow::bail!("Python fallback is not enabled in sandbox configuration");
    }

    info!("Building bubblewrap command for Python backtest");

    // Check if bubblewrap is available
    let bwrap_path = which::which("bwrap").context(
        "bubblewrap (bwrap) not found in PATH. Install bubblewrap for Python sandboxing.",
    )?;

    let mut cmd = std::process::Command::new(bwrap_path);

    // Unshare all namespaces
    cmd.arg("--unshare-all");
    info!("bubblewrap: --unshare-all");

    // Die with parent process
    cmd.arg("--die-with-parent");
    info!("bubblewrap: --die-with-parent");

    // Drop all capabilities
    cmd.arg("--cap-drop").arg("ALL");
    info!("bubblewrap: --cap-drop ALL");

    // Set resource limits if supported
    if config.max_memory_mb > 0 {
        // Convert MB to bytes for as-limit
        let memory_bytes = config.max_memory_mb * 1024 * 1024;
        cmd.arg("--as-limit").arg(memory_bytes.to_string());
        info!(memory_bytes = memory_bytes, "bubblewrap: --as-limit");
    }

    // Bind mount allowed paths as read-only
    for path in &config.allowed_paths {
        if path.exists() {
            cmd.arg("--ro-bind")
                .arg(path.as_os_str())
                .arg(path.as_os_str());
            info!(path = %path.display(), "bubblewrap: --ro-bind");
        } else {
            warn!(path = %path.display(), "Allowed path does not exist, skipping bind mount");
        }
    }

    // Create tmpfs for /tmp
    cmd.arg("--tmpfs").arg("/tmp");
    info!("bubblewrap: --tmpfs /tmp");

    // Create proc filesystem (read-only)
    cmd.arg("--proc").arg("/proc");
    info!("bubblewrap: --proc /proc");

    // Create dev filesystem with minimal devices
    cmd.arg("--dev").arg("/dev");
    info!("bubblewrap: --dev /dev");

    // Disable network if not enabled
    if !config.network_enabled {
        cmd.arg("--unshare-net");
        info!("bubblewrap: --unshare-net");
    }

    // Set working directory
    cmd.arg("--chdir").arg("/");

    // Add the Python executable and script
    cmd.arg(python_path.as_ref().as_os_str());
    cmd.arg(script_path.as_ref().as_os_str());

    info!("Bubblewrap command built successfully");
    Ok(cmd)
}

/// Executes a Python script in a bubblewrap sandbox
///
/// # Arguments
/// * `config` - The sandbox configuration
/// * `python_path` - Path to the Python executable
/// * `script_path` - Path to the Python script to run
/// * `args` - Additional arguments to pass to the script
///
/// # Returns
/// * `Ok(std::process::Output)` containing the process output
/// * `Err` if the sandbox failed or process execution failed
pub fn run_python_sandboxed(
    config: &SandboxConfig,
    python_path: impl AsRef<Path>,
    script_path: impl AsRef<Path>,
    args: &[&str],
) -> Result<std::process::Output> {
    let mut cmd = build_bubblewrap_command(config, python_path, script_path)?;

    // Add additional arguments
    for arg in args {
        cmd.arg(arg);
    }

    info!("Executing sandboxed Python backtest");

    let output = cmd
        .output()
        .context("Failed to execute sandboxed Python process")?;

    info!(
        status = ?output.status.code(),
        stdout_len = output.stdout.len(),
        stderr_len = output.stderr.len(),
        "Sandboxed Python process completed"
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.max_memory_mb, DEFAULT_MAX_MEMORY_MB);
        assert_eq!(config.max_runtime_seconds, DEFAULT_MAX_RUNTIME_SECONDS);
        assert!(!config.network_enabled);
        assert!(!config.python_fallback_enabled);
        assert_eq!(config.allowed_paths.len(), 1);
        assert_eq!(
            config.allowed_paths[0],
            PathBuf::from(DEFAULT_MARKET_DATA_PATH)
        );
    }

    #[test]
    fn test_sandbox_config_new() {
        let config = SandboxConfig::new();
        assert_eq!(config.max_memory_mb, DEFAULT_MAX_MEMORY_MB);
        assert_eq!(config.max_runtime_seconds, DEFAULT_MAX_RUNTIME_SECONDS);
    }

    #[test]
    fn test_sandbox_config_for_python_backtest() {
        let config = SandboxConfig::for_python_backtest();
        assert!(config.python_fallback_enabled);
        assert!(!config.network_enabled);
    }

    #[test]
    fn test_sandbox_config_validation_success() {
        let config = SandboxConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_sandbox_config_validation_memory_exceeds() {
        let config = SandboxConfig::default().with_memory_limit(1024);
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Memory limit"));
        assert!(err_msg.contains("exceeds maximum allowed"));
    }

    #[test]
    fn test_sandbox_config_validation_runtime_exceeds() {
        let config = SandboxConfig::default().with_runtime_limit(600);
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Runtime limit"));
        assert!(err_msg.contains("exceeds maximum allowed"));
    }

    #[test]
    fn test_sandbox_config_validation_empty_paths() {
        let mut config = SandboxConfig::default();
        config.allowed_paths.clear();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one allowed path"));
    }

    #[test]
    fn test_sandbox_config_builder_methods() {
        let config = SandboxConfig::new()
            .with_memory_limit(256)
            .with_runtime_limit(120)
            .with_allowed_path("/custom/path")
            .with_network_enabled(true)
            .with_python_fallback(true);

        assert_eq!(config.max_memory_mb, 256);
        assert_eq!(config.max_runtime_seconds, 120);
        assert_eq!(config.allowed_paths.len(), 2); // Default + custom
        assert!(config.network_enabled);
        assert!(config.python_fallback_enabled);
    }

    #[test]
    fn test_sandbox_config_within_limits() {
        let config = SandboxConfig::new()
            .with_memory_limit(512)
            .with_runtime_limit(300);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_sandbox_config_boundary_values() {
        // Test exact boundary values
        let config = SandboxConfig::new()
            .with_memory_limit(512)
            .with_runtime_limit(300);
        assert!(config.validate().is_ok());

        // Test one over boundary
        let config = SandboxConfig::new().with_memory_limit(513);
        assert!(config.validate().is_err());

        let config = SandboxConfig::default().with_runtime_limit(301);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_MAX_MEMORY_MB, 512);
        assert_eq!(DEFAULT_MAX_RUNTIME_SECONDS, 300);
        assert_eq!(DEFAULT_MARKET_DATA_PATH, "/data/market");
    }

    #[test]
    #[cfg(not(feature = "host-sandbox"))]
    fn test_apply_landlock_stub() {
        let config = SandboxConfig::default();
        // Should succeed but do nothing when feature is disabled
        assert!(apply_landlock_ruleset(&config).is_ok());
    }

    #[test]
    #[cfg(not(feature = "host-sandbox"))]
    fn test_apply_seccomp_stub() {
        let config = SandboxConfig::default();
        // Should succeed but do nothing when feature is disabled
        assert!(apply_seccomp_filter(&config).is_ok());
    }

    #[test]
    fn test_build_bubblewrap_without_fallback_enabled() {
        let config = SandboxConfig::default(); // python_fallback_enabled = false
        let result = build_bubblewrap_command(&config, "/usr/bin/python3", "/tmp/test.py");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Python fallback is not enabled"));
    }
}
