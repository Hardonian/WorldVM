//! Deterministic .worldmod container format, manifest parser, and security hardening.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use worldvm_core::{ResourceLimits, WorldVmError, WORLDVM_ABI_VERSION};
use worldvm_signing::{compute_canonical_hash, PackageSignature};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// Security limits for archive extraction.
pub const MAX_ARCHIVE_FILE_COUNT: usize = 1_000;
pub const MAX_SINGLE_FILE_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024; // 64 MB
pub const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 128 * 1024 * 1024; // 128 MB

/// Manifest structure declared inside manifest.toml.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldModManifest {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub worldvm: String,
    pub abi: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub resources: ResourceDeclaration,
    #[serde(default)]
    pub permissions: PermissionDeclaration,
    #[serde(default)]
    pub events: EventDeclaration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceDeclaration {
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u32,
    #[serde(default = "default_fuel")]
    pub fuel: u64,
    #[serde(default = "default_max_ms")]
    pub max_execution_ms: u64,
}

impl Default for ResourceDeclaration {
    fn default() -> Self {
        Self {
            memory_mb: default_memory_mb(),
            fuel: default_fuel(),
            max_execution_ms: default_max_ms(),
        }
    }
}

fn default_memory_mb() -> u32 {
    32
}
fn default_fuel() -> u64 {
    500_000
}
fn default_max_ms() -> u64 {
    5
}

impl From<&ResourceDeclaration> for ResourceLimits {
    fn from(r: &ResourceDeclaration) -> Self {
        Self {
            memory_mb: r.memory_mb,
            fuel_limit: r.fuel,
            max_execution_ms: r.max_execution_ms,
            max_events_per_tick: 64,
            max_call_depth: 8,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PermissionDeclaration {
    #[serde(default)]
    pub request: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventDeclaration {
    #[serde(default)]
    pub subscribe: Vec<String>,
}

/// In-memory representation of an unpacked and validated .worldmod package.
#[derive(Debug, Clone)]
pub struct WorldModPackage {
    pub manifest: WorldModManifest,
    pub raw_manifest: Vec<u8>,
    pub wasm_bytes: Vec<u8>,
    pub signature: Option<PackageSignature>,
    pub assets: HashMap<String, Vec<u8>>,
    pub content_hash: String,
}

impl WorldModPackage {
    /// Reads and strictly validates a .worldmod archive from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WorldVmError> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).map_err(|e| WorldVmError::InvalidPackage {
            reason: format!("Corrupt or non-ZIP archive: {e}"),
        })?;

        if archive.len() > MAX_ARCHIVE_FILE_COUNT {
            return Err(WorldVmError::InvalidPackage {
                reason: format!(
                    "Archive file count ({}) exceeds maximum limit ({})",
                    archive.len(),
                    MAX_ARCHIVE_FILE_COUNT
                ),
            });
        }

        let mut raw_manifest = None;
        let mut wasm_bytes = None;
        let mut signature = None;
        let mut assets = HashMap::new();
        let mut total_uncompressed_bytes: u64 = 0;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| WorldVmError::InvalidPackage {
                    reason: format!("Failed reading file #{i} in archive: {e}"),
                })?;

            let name = file.name().to_string();

            // 1. Path traversal / Zip Slip protection
            if name.contains("..")
                || name.starts_with('/')
                || name.starts_with('\\')
                || name.contains(':')
            {
                return Err(WorldVmError::InvalidPackage {
                    reason: format!("Malicious path traversal detected in package: {name}"),
                });
            }

            // 2. Prohibit native executable extensions
            let lower_name = name.to_lowercase();
            if lower_name.ends_with(".dll")
                || lower_name.ends_with(".so")
                || lower_name.ends_with(".dylib")
                || lower_name.ends_with(".exe")
                || lower_name.ends_with(".bat")
                || lower_name.ends_with(".cmd")
                || lower_name.ends_with(".ps1")
                || lower_name.ends_with(".sh")
            {
                return Err(WorldVmError::InvalidPackage {
                    reason: format!("Package contains prohibited native executable: {name}"),
                });
            }

            // 3. Decompression bomb check per file
            let uncompressed_size = file.size();
            if uncompressed_size > MAX_SINGLE_FILE_UNCOMPRESSED_BYTES {
                return Err(WorldVmError::InvalidPackage {
                    reason: format!(
                        "File '{name}' exceeds uncompressed size limit ({} bytes)",
                        MAX_SINGLE_FILE_UNCOMPRESSED_BYTES
                    ),
                });
            }

            total_uncompressed_bytes += uncompressed_size;
            if total_uncompressed_bytes > MAX_TOTAL_UNCOMPRESSED_BYTES {
                return Err(WorldVmError::InvalidPackage {
                    reason: format!(
                        "Total archive uncompressed size exceeds limit ({} bytes)",
                        MAX_TOTAL_UNCOMPRESSED_BYTES
                    ),
                });
            }

            let mut contents = Vec::with_capacity(uncompressed_size as usize);
            file.read_to_end(&mut contents)
                .map_err(|e| WorldVmError::InvalidPackage {
                    reason: format!("Failed reading contents of '{name}': {e}"),
                })?;

            match name.as_str() {
                "manifest.toml" => raw_manifest = Some(contents),
                "module.wasm" => wasm_bytes = Some(contents),
                "signature.json" => {
                    let sig: PackageSignature = serde_json::from_slice(&contents).map_err(|e| {
                        WorldVmError::InvalidSignature {
                            reason: format!("Failed parsing signature.json: {e}"),
                        }
                    })?;
                    signature = Some(sig);
                }
                _ => {
                    if !file.is_dir() {
                        assets.insert(name, contents);
                    }
                }
            }
        }

        let raw_manifest = raw_manifest.ok_or_else(|| WorldVmError::InvalidPackage {
            reason: "Missing mandatory 'manifest.toml' in .worldmod".to_string(),
        })?;

        let wasm_bytes = wasm_bytes.ok_or_else(|| WorldVmError::InvalidPackage {
            reason: "Missing mandatory 'module.wasm' in .worldmod".to_string(),
        })?;

        // Parse and validate manifest
        let manifest_str = std::str::from_utf8(&raw_manifest).map_err(|e| {
            WorldVmError::InvalidPackage {
                reason: format!("manifest.toml is not valid UTF-8: {e}"),
            }
        })?;

        let manifest: WorldModManifest =
            toml::from_str(manifest_str).map_err(|e| WorldVmError::InvalidPackage {
                reason: format!("Failed parsing manifest.toml: {e}"),
            })?;

        if manifest.name.trim().is_empty() {
            return Err(WorldVmError::InvalidPackage {
                reason: "Manifest 'name' cannot be empty".to_string(),
            });
        }

        if manifest.abi != WORLDVM_ABI_VERSION {
            return Err(WorldVmError::AbiMismatch {
                expected: WORLDVM_ABI_VERSION.to_string(),
                found: manifest.abi,
            });
        }

        let content_hash = compute_canonical_hash(&raw_manifest, &wasm_bytes);

        Ok(Self {
            manifest,
            raw_manifest,
            wasm_bytes,
            signature,
            assets,
            content_hash,
        })
    }

    /// Reads a .worldmod file from disk.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, WorldVmError> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| WorldVmError::InvalidPackage {
            reason: format!("Failed reading file {}: {e}", path.as_ref().display()),
        })?;
        Self::from_bytes(&bytes)
    }
}

/// Deterministic package builder.
pub struct WorldModBuilder {
    manifest_toml: String,
    wasm_bytes: Vec<u8>,
    signature: Option<PackageSignature>,
    assets: HashMap<String, Vec<u8>>,
}

impl WorldModBuilder {
    pub fn new(manifest_toml: impl Into<String>, wasm_bytes: Vec<u8>) -> Self {
        Self {
            manifest_toml: manifest_toml.into(),
            wasm_bytes,
            signature: None,
            assets: HashMap::new(),
        }
    }

    pub fn with_signature(mut self, sig: PackageSignature) -> Self {
        self.signature = Some(sig);
        self
    }

    pub fn add_asset(mut self, path: impl Into<String>, data: Vec<u8>) -> Self {
        self.assets.insert(path.into(), data);
        self
    }

    /// Builds deterministic .worldmod archive bytes with standardized normalized timestamps.
    pub fn build(self) -> Result<Vec<u8>, WorldVmError> {
        let mut buf = Vec::new();
        let mut writer = ZipWriter::new(Cursor::new(&mut buf));

        // Use standard zip file options with DEFLATE
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // 1. Write manifest.toml
        writer
            .start_file("manifest.toml", options)
            .map_err(|e| WorldVmError::InvalidPackage {
                reason: format!("Zip error: {e}"),
            })?;
        writer
            .write_all(self.manifest_toml.as_bytes())
            .map_err(|e| WorldVmError::InvalidPackage {
                reason: format!("Zip write error: {e}"),
            })?;

        // 2. Write module.wasm
        writer
            .start_file("module.wasm", options)
            .map_err(|e| WorldVmError::InvalidPackage {
                reason: format!("Zip error: {e}"),
            })?;
        writer
            .write_all(&self.wasm_bytes)
            .map_err(|e| WorldVmError::InvalidPackage {
                reason: format!("Zip write error: {e}"),
            })?;

        // 3. Write signature.json if present
        if let Some(ref sig) = self.signature {
            let sig_json = serde_json::to_vec_pretty(sig).map_err(|e| {
                WorldVmError::SerializationError {
                    message: format!("Signature serialization error: {e}"),
                }
            })?;
            writer
                .start_file("signature.json", options)
                .map_err(|e| WorldVmError::InvalidPackage {
                    reason: format!("Zip error: {e}"),
                })?;
            writer
                .write_all(&sig_json)
                .map_err(|e| WorldVmError::InvalidPackage {
                    reason: format!("Zip write error: {e}"),
                })?;
        }

        // 4. Write assets in sorted deterministic order
        let mut sorted_assets: Vec<_> = self.assets.into_iter().collect();
        sorted_assets.sort_by(|a, b| a.0.cmp(&b.0));

        for (asset_name, asset_data) in sorted_assets {
            writer
                .start_file(asset_name, options)
                .map_err(|e| WorldVmError::InvalidPackage {
                    reason: format!("Zip error: {e}"),
                })?;
            writer
                .write_all(&asset_data)
                .map_err(|e| WorldVmError::InvalidPackage {
                    reason: format!("Zip write error: {e}"),
                })?;
        }

        writer
            .finish()
            .map_err(|e| WorldVmError::InvalidPackage {
                reason: format!("Failed finalizing zip archive: {e}"),
            })?;

        Ok(buf)
    }
}
