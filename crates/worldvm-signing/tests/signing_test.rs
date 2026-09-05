use std::collections::HashSet;
use worldvm_core::WorldVmError;
use worldvm_signing::{
    compute_canonical_hash, generate_keypair, sign_content, verify_package_signature, TrustLevel,
};

#[test]
fn test_canonical_hashing_determinism() {
    let manifest = b"name = \"test\"\nversion = \"1.0.0\"";
    let wasm = b"\x00asm\x01\x00\x00\x00";

    let hash1 = compute_canonical_hash(manifest, wasm);
    let hash2 = compute_canonical_hash(manifest, wasm);

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA-256 hex string

    // Any modification changes hash
    let modified_manifest = b"name = \"test2\"\nversion = \"1.0.0\"";
    let hash3 = compute_canonical_hash(modified_manifest, wasm);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_ed25519_sign_and_verify() {
    let (signing_key, verifying_key) = generate_keypair();
    let content_hash = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

    let signature = sign_content(&signing_key, content_hash);
    assert_eq!(signature.content_hash, content_hash);

    let empty_studio_keys = HashSet::new();
    let trust = verify_package_signature(&signature, content_hash, TrustLevel::Signed, &empty_studio_keys)
        .expect("Signature must verify");
    assert_eq!(trust, TrustLevel::Signed);

    // Studio approved verification
    let mut studio_keys = HashSet::new();
    studio_keys.insert(hex::encode(verifying_key.as_bytes()));
    let studio_trust = verify_package_signature(&signature, content_hash, TrustLevel::StudioApproved, &studio_keys)
        .expect("Studio approved signature must verify");
    assert_eq!(studio_trust, TrustLevel::StudioApproved);
}

#[test]
fn test_signature_tampering_rejected() {
    let (signing_key, _verifying_key) = generate_keypair();
    let content_hash = "1111111111111111111111111111111111111111111111111111111111111111";
    let mut signature = sign_content(&signing_key, content_hash);

    // Tamper with content hash
    let empty_keys = HashSet::new();
    let res = verify_package_signature(&signature, "2222222222222222222222222222222222222222222222222222222222222222", TrustLevel::Signed, &empty_keys);
    assert!(matches!(res, Err(WorldVmError::InvalidSignature { .. })));

    // Tamper with signature bytes
    signature.signature = "00".repeat(64);
    let res2 = verify_package_signature(&signature, content_hash, TrustLevel::Signed, &empty_keys);
    assert!(matches!(res2, Err(WorldVmError::InvalidSignature { .. })));
}
