use std::{
    collections::BTreeMap,
    fs,
    path::{Component as PathComponent, Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};

wasmtime::component::bindgen!({
    path: "../../wit",
    world: "datahub-plugin",
});

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;
static PLUGIN_RUNS: AtomicU64 = AtomicU64::new(0);
static PLUGIN_TRAPS: AtomicU64 = AtomicU64::new(0);
static PLUGIN_QUOTA_REJECTIONS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PluginMetrics {
    pub runs: u64,
    pub traps: u64,
    pub quota_rejections: u64,
}

#[must_use]
pub fn plugin_metrics() -> PluginMetrics {
    PluginMetrics {
        runs: PLUGIN_RUNS.load(Ordering::Relaxed),
        traps: PLUGIN_TRAPS.load(Ordering::Relaxed),
        quota_rejections: PLUGIN_QUOTA_REJECTIONS.load(Ordering::Relaxed),
    }
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin package is malformed: {0}")]
    MalformedPackage(String),
    #[error("plugin manifest is invalid: {0}")]
    InvalidManifest(String),
    #[error("plugin component hash does not match the manifest")]
    HashMismatch,
    #[error("plugin input path is unsafe: {0}")]
    UnsafePath(String),
    #[error("plugin input was not declared: {0}")]
    UndeclaredInput(String),
    #[error("plugin input exceeds its byte quota")]
    InputQuotaExceeded,
    #[error("plugin output exceeds its byte quota")]
    OutputQuotaExceeded,
    #[error("plugin execution failed: {0}")]
    Execution(String),
    #[error("plugin returned an error: {0}")]
    Guest(String),
    #[error("plugin installation conflicts with an existing version")]
    InstallConflict,
    #[error("plugin version is not installed")]
    NotInstalled,
    #[error("plugin filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("plugin request serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub read_inputs: Vec<String>,
    pub write_output_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PluginLimits {
    pub fuel: u64,
    pub memory_bytes: usize,
    pub timeout_ms: u64,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
}

impl Default for PluginLimits {
    fn default() -> Self {
        Self {
            fuel: 10_000_000,
            memory_bytes: 64 * 1024 * 1024,
            timeout_ms: 2_000,
            max_input_bytes: 8 * 1024 * 1024,
            max_output_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub id: String,
    pub version: Version,
    pub api_version: Version,
    pub component: String,
    pub sha256: String,
    pub output_file: String,
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub limits: PluginLimits,
}

impl PluginManifest {
    /// Validates identifiers, compatibility, capabilities, paths, and quotas.
    ///
    /// # Errors
    /// Returns an invalid-manifest error for unsafe or unsupported declarations.
    pub fn validate(&self) -> Result<(), PluginError> {
        if !is_safe_identifier(&self.id) {
            return Err(PluginError::InvalidManifest("unsafe plugin id".into()));
        }
        let supported = Version::new(1, 0, 0);
        if self.api_version.major != supported.major || self.api_version > supported {
            return Err(PluginError::InvalidManifest(format!(
                "unsupported API version {}",
                self.api_version
            )));
        }
        validate_relative_path(&self.component)?;
        if Path::new(&self.component).components().count() != 1 {
            return Err(PluginError::InvalidManifest(
                "component must be a package-root file".into(),
            ));
        }
        if self.sha256.len() != 64 || !self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(PluginError::InvalidManifest(
                "sha256 must contain 64 hexadecimal characters".into(),
            ));
        }
        validate_relative_path(&self.capabilities.write_output_directory)?;
        validate_relative_path(&self.output_file)?;
        if Path::new(&self.output_file).components().count() != 1 {
            return Err(PluginError::InvalidManifest(
                "output_file must be a file name".into(),
            ));
        }
        for input in &self.capabilities.read_inputs {
            validate_relative_path(input)?;
        }
        let mut inputs = self.capabilities.read_inputs.clone();
        inputs.sort();
        inputs.dedup();
        if inputs.len() != self.capabilities.read_inputs.len() {
            return Err(PluginError::InvalidManifest(
                "read_inputs contains duplicates".into(),
            ));
        }
        if self.limits.fuel == 0
            || self.limits.memory_bytes < 1024 * 1024
            || self.limits.timeout_ms == 0
            || self.limits.max_input_bytes == 0
            || self.limits.max_output_bytes == 0
        {
            return Err(PluginError::InvalidManifest(
                "all resource quotas must be positive and memory must be at least 1 MiB".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PluginPackage {
    pub root: PathBuf,
    pub manifest: PluginManifest,
    pub component: Vec<u8>,
}

impl PluginPackage {
    /// Loads and verifies a plugin directory containing `plugin.toml` and its component.
    ///
    /// # Errors
    /// Returns package, manifest, hash, quota, or filesystem errors.
    pub fn load(root: impl AsRef<Path>) -> Result<Self, PluginError> {
        let root = root.as_ref();
        let manifest_path = root.join("plugin.toml");
        let metadata = fs::symlink_metadata(&manifest_path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAX_MANIFEST_BYTES
        {
            return Err(PluginError::MalformedPackage(
                "plugin.toml is missing, not a file, or too large".into(),
            ));
        }
        let manifest_text = fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_text)
            .map_err(|error| PluginError::MalformedPackage(error.to_string()))?;
        manifest.validate()?;
        let component_path = root.join(&manifest.component);
        let component_metadata = fs::symlink_metadata(&component_path)?;
        if component_metadata.file_type().is_symlink()
            || !component_metadata.is_file()
            || component_metadata.len() > MAX_COMPONENT_BYTES
        {
            return Err(PluginError::MalformedPackage(
                "component is missing, not a file, or too large".into(),
            ));
        }
        let component = fs::read(component_path)?;
        let digest = format!("{:x}", Sha256::digest(&component));
        if !digest.eq_ignore_ascii_case(&manifest.sha256) {
            return Err(PluginError::HashMismatch);
        }
        Ok(Self {
            root: root.to_path_buf(),
            manifest,
            component,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRunRequest {
    pub inputs: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginOutput {
    pub path: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PluginRegistry {
    root: PathBuf,
}

impl PluginRegistry {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Installs an immutable, hash-verified plugin version.
    ///
    /// Reinstalling identical bytes is idempotent; differing bytes for the same
    /// ID and version are rejected.
    ///
    /// # Errors
    /// Returns validation, conflict, or filesystem errors.
    pub fn install(&self, package: &PluginPackage) -> Result<PathBuf, PluginError> {
        let target = self
            .root
            .join(&package.manifest.id)
            .join(package.manifest.version.to_string());
        if target.exists() {
            let installed = PluginPackage::load(&target)?;
            if installed.manifest.sha256 == package.manifest.sha256 {
                return Ok(target);
            }
            return Err(PluginError::InstallConflict);
        }
        fs::create_dir_all(&target)?;
        let mut manifest = package.manifest.clone();
        manifest.component = "plugin.wasm".into();
        fs::write(target.join("plugin.wasm"), &package.component)?;
        let encoded = toml::to_string_pretty(&manifest)
            .map_err(|error| PluginError::InvalidManifest(error.to_string()))?;
        fs::write(target.join("plugin.toml"), encoded)?;
        Ok(target)
    }

    /// Loads one exact installed plugin version.
    ///
    /// # Errors
    /// Returns not-installed, validation, hash, or filesystem errors.
    pub fn load(&self, id: &str, version: &Version) -> Result<PluginPackage, PluginError> {
        if !is_safe_identifier(id) {
            return Err(PluginError::NotInstalled);
        }
        let root = self.root.join(id).join(version.to_string());
        if !root.is_dir() {
            return Err(PluginError::NotInstalled);
        }
        PluginPackage::load(root)
    }
}

/// Executes a component with only the WIT payload capability and bounded resources.
///
/// No WASI context is linked, so the guest has no filesystem, environment,
/// credential, clock, random, socket, or network capability.
///
/// # Errors
/// Returns declaration, quota, compilation, instantiation, trap, timeout, fuel,
/// memory, or guest errors.
pub fn run_plugin(
    package: &PluginPackage,
    request: &PluginRunRequest,
) -> Result<PluginOutput, PluginError> {
    PLUGIN_RUNS.fetch_add(1, Ordering::Relaxed);
    let result = run_plugin_inner(package, request);
    if let Err(error) = &result {
        match error {
            PluginError::InputQuotaExceeded | PluginError::OutputQuotaExceeded => {
                PLUGIN_QUOTA_REJECTIONS.fetch_add(1, Ordering::Relaxed);
            }
            PluginError::Execution(_) | PluginError::Guest(_) => {
                PLUGIN_TRAPS.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }
    result
}

fn run_plugin_inner(
    package: &PluginPackage,
    request: &PluginRunRequest,
) -> Result<PluginOutput, PluginError> {
    validate_inputs(&package.manifest, request)?;
    let payload = serde_json::to_vec(request)?;
    if payload.len() > package.manifest.limits.max_input_bytes {
        return Err(PluginError::InputQuotaExceeded);
    }

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.consume_fuel(true);
    config.epoch_interruption(true);
    let engine = Engine::new(&config).map_err(execution_error)?;
    let component = Component::new(&engine, &package.component).map_err(execution_error)?;
    let linker = Linker::new(&engine);
    let limits = StoreLimitsBuilder::new()
        .memory_size(package.manifest.limits.memory_bytes)
        .instances(1)
        .memories(1)
        .build();
    let mut store = Store::new(&engine, HostState { limits });
    store.limiter(|state| &mut state.limits);
    store
        .set_fuel(package.manifest.limits.fuel)
        .map_err(execution_error)?;
    store.set_epoch_deadline(1);
    store.epoch_deadline_trap();
    let (cancel, deadline) = mpsc::channel::<()>();
    let deadline_engine = engine.clone();
    let timeout = Duration::from_millis(package.manifest.limits.timeout_ms);
    thread::spawn(move || {
        if deadline.recv_timeout(timeout).is_err() {
            deadline_engine.increment_epoch();
        }
    });

    let bindings =
        DatahubPlugin::instantiate(&mut store, &component, &linker).map_err(execution_error)?;
    let result = bindings
        .call_run(&mut store, &payload)
        .map_err(execution_error)?;
    drop(cancel);
    let content = result.map_err(PluginError::Guest)?;
    if content.len() > package.manifest.limits.max_output_bytes {
        return Err(PluginError::OutputQuotaExceeded);
    }
    let path = format!(
        "{}/{}",
        package
            .manifest
            .capabilities
            .write_output_directory
            .trim_end_matches('/'),
        package.manifest.output_file
    );
    validate_relative_path(&path)?;
    Ok(PluginOutput { path, content })
}

struct HostState {
    limits: StoreLimits,
}

fn validate_inputs(
    manifest: &PluginManifest,
    request: &PluginRunRequest,
) -> Result<(), PluginError> {
    let mut total = 0_usize;
    for (path, content) in &request.inputs {
        validate_relative_path(path)?;
        if !manifest
            .capabilities
            .read_inputs
            .iter()
            .any(|declared| declared == path)
        {
            return Err(PluginError::UndeclaredInput(path.clone()));
        }
        total = total
            .checked_add(content.len())
            .ok_or(PluginError::InputQuotaExceeded)?;
        if total > manifest.limits.max_input_bytes {
            return Err(PluginError::InputQuotaExceeded);
        }
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), PluginError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || path.components().any(|component| {
            !matches!(component, PathComponent::Normal(_))
                || component.as_os_str().to_string_lossy().is_empty()
        })
    {
        return Err(PluginError::UnsafePath(value.into()));
    }
    Ok(())
}

fn is_safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn execution_error(error: impl std::fmt::Display) -> PluginError {
    PluginError::Execution(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use semver::Version;
    use sha2::{Digest, Sha256};
    use tempfile::tempdir;

    use super::{
        PluginCapabilities, PluginError, PluginLimits, PluginManifest, PluginPackage,
        PluginRegistry, PluginRunRequest, validate_inputs,
    };

    fn manifest() -> PluginManifest {
        PluginManifest {
            id: "echo-plugin".into(),
            version: Version::new(1, 2, 3),
            api_version: Version::new(1, 0, 0),
            component: "plugin.wasm".into(),
            sha256: "0".repeat(64),
            output_file: "result.bin".into(),
            capabilities: PluginCapabilities {
                read_inputs: vec!["input/data.bin".into()],
                write_output_directory: "generated/echo".into(),
            },
            limits: PluginLimits::default(),
        }
    }

    #[test]
    fn rejects_traversal_and_undeclared_inputs() {
        let manifest = manifest();
        let traversal = PluginRunRequest {
            inputs: BTreeMap::from([("../secret".into(), Vec::new())]),
        };
        assert!(matches!(
            validate_inputs(&manifest, &traversal),
            Err(PluginError::UnsafePath(_))
        ));
        let undeclared = PluginRunRequest {
            inputs: BTreeMap::from([("input/other.bin".into(), Vec::new())]),
        };
        assert!(matches!(
            validate_inputs(&manifest, &undeclared),
            Err(PluginError::UndeclaredInput(_))
        ));
    }

    #[test]
    fn enforces_input_quota_before_execution() {
        let mut manifest = manifest();
        manifest.limits.max_input_bytes = 4;
        let request = PluginRunRequest {
            inputs: BTreeMap::from([("input/data.bin".into(), vec![0; 5])]),
        };
        assert!(matches!(
            validate_inputs(&manifest, &request),
            Err(PluginError::InputQuotaExceeded)
        ));
    }

    #[test]
    fn verifies_hash_and_installs_exact_version_idempotently() {
        let source = tempdir().expect("source tempdir");
        let registry_root = tempdir().expect("registry tempdir");
        let component = b"not-yet-a-component";
        fs::write(source.path().join("plugin.wasm"), component).expect("component");
        let mut manifest = manifest();
        manifest.sha256 = format!("{:x}", Sha256::digest(component));
        fs::write(
            source.path().join("plugin.toml"),
            toml::to_string_pretty(&manifest).expect("manifest TOML"),
        )
        .expect("manifest");
        let package = PluginPackage::load(source.path()).expect("verified package");
        let registry = PluginRegistry::new(registry_root.path());
        let first = registry.install(&package).expect("first install");
        let second = registry.install(&package).expect("idempotent install");
        assert_eq!(first, second);
        let installed = registry
            .load(&manifest.id, &manifest.version)
            .expect("exact version");
        assert_eq!(installed.manifest.sha256, manifest.sha256);
    }

    #[test]
    fn rejects_malformed_manifest_and_hash_mismatch() {
        let root = tempdir().expect("package tempdir");
        fs::write(root.path().join("plugin.toml"), "not = [valid").expect("malformed");
        assert!(matches!(
            PluginPackage::load(root.path()),
            Err(PluginError::MalformedPackage(_))
        ));

        fs::write(root.path().join("plugin.wasm"), b"different").expect("component");
        fs::write(
            root.path().join("plugin.toml"),
            toml::to_string_pretty(&manifest()).expect("manifest TOML"),
        )
        .expect("manifest");
        assert!(matches!(
            PluginPackage::load(root.path()),
            Err(PluginError::HashMismatch)
        ));
    }
}
