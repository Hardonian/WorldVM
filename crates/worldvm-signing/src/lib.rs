//! Package signing, verification, and canonical content hashing using Ed25519.

use std::collections::HashSet;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worldvm_core::WorldVmError;

/// Trust level classification for a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    Unsigned = 0,
    Signed = 1,
    StudioApproved = 2,
    RegistryVerified = 3,
}

/// Serialized package signature format stored in signature.json.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageSignature {
    pub algorithm: String,
    pub public_key: String,
    pub signature: String,
    pub content_hash: String,
    pub timestamp: u64,
}

/// Generates a new random Ed25519 signing keypair.
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Canonical package content hash calculation over manifest, wasm binary, and optional assets.
pub fn compute_canonical_hash(manifest_bytes: &[u8], wasm_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"WORLDVM-CONTENT-V1");
    hasher.update((manifest_bytes.len() as u64).to_le_bytes());
    hasher.update(manifest_bytes);
    hasher.update((wasm_bytes.len() as u64).to_le_bytes());
    hasher.update(wasm_bytes);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Signs a package's content hash with an Ed25519 signing key.
pub fn sign_content(signing_key: &SigningKey, content_hash: &str) -> PackageSignature {
    let hash_bytes = content_hash.as_bytes();
    let signature: Signature = signing_key.sign(hash_bytes);
    let public_key = signing_key.verifying_key();

    PackageSignature {
        algorithm: "ed25519".to_string(),
        public_key: hex::encode(public_key.as_bytes()),
        signature: hex::encode(signature.to_bytes()),
        content_hash: content_hash.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

/// Verifies a package signature against the expected content hash and trusted public keys.
pub fn verify_package_signature(
    sig: &PackageSignature,
    expected_content_hash: &str,
    required_trust: TrustLevel,
    trusted_studio_keys: &HashSet<String>,
) -> Result<TrustLevel, WorldVmError> {
    if sig.algorithm != "ed25519" {
        return Err(WorldVmError::InvalidSignature {
            reason: format!("Unsupported signature algorithm: {}", sig.algorithm),
        });
    }

    if sig.content_hash != expected_content_hash {
        return Err(WorldVmError::InvalidSignature {
            reason: format!(
                "Content hash mismatch: signature hash is {}, calculated hash is {}",
                sig.content_hash, expected_content_hash
            ),
        });
    }

    let pub_key_bytes = hex::decode(&sig.public_key).map_err(|e| WorldVmError::InvalidSignature {
        reason: format!("Invalid public key hex: {e}"),
    })?;

    let pub_key_arr: [u8; 32] = pub_key_bytes
        .try_into()
        .map_err(|_| WorldVmError::InvalidSignature {
            reason: "Public key must be 32 bytes".to_string(),
        })?;

    let verifying_key = VerifyingKey::from_bytes(&pub_key_arr).map_err(|e| {
        WorldVmError::InvalidSignature {
            reason: format!("Invalid Ed25519 public key: {e}"),
        }
    })?;

    let sig_bytes = hex::decode(&sig.signature).map_err(|e| WorldVmError::InvalidSignature {
        reason: format!("Invalid signature hex: {e}"),
    })?;

    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| WorldVmError::InvalidSignature {
            reason: "Signature must be 64 bytes".to_string(),
        })?;

    let signature = Signature::from_bytes(&sig_arr);

    verifying_key
        .verify(sig.content_hash.as_bytes(), &signature)
        .map_err(|e| WorldVmError::InvalidSignature {
            reason: format!("Signature verification failed: {e}"),
        })?;

    // Determine trust classification
    let effective_trust = if trusted_studio_keys.contains(&sig.public_key) {
        TrustLevel::StudioApproved
    } else {
        TrustLevel::Signed
    };

    if effective_trust < required_trust {
        return Err(WorldVmError::PermissionDenied {
            capability: "package.trust".to_string(),
            reason: format!(
                "Package trust level {:?} does not satisfy required policy {:?}",
                effective_trust, required_trust
            ),
        });
    }

    Ok(effective_trust)
}
