//! Malformed Package Corpus: Zip slip, prohibited binaries, corrupted WASM, tampered signatures.

use std::io::{Cursor, Write};
use worldvm_package::WorldModBuilder;
use worldvm_signing::{generate_keypair, sign_content};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub fn build_zip_slip_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buf));
    let options = SimpleFileOptions::default();

    writer.start_file("../../../root/shadow", options).unwrap();
    writer.write_all(b"malicious_slip").unwrap();
    writer.finish().unwrap();
    buf
}

pub fn build_native_binary_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buf));
    let options = SimpleFileOptions::default();

    writer.start_file("manifest.toml", options).unwrap();
    writer.write_all(b"name = \"bad\"\nversion = \"1.0.0\"\npublisher = \"x\"\nworldvm = \"1\"\nabi = \"1.0\"").unwrap();
    writer.start_file("module.wasm", options).unwrap();
    writer.write_all(b"\x00asm\x01\x00\x00\x00").unwrap();
    writer.start_file("malicious.dll", options).unwrap();
    writer.write_all(b"MZ\x90\x00\x03\x00\x00\x00").unwrap();
    writer.finish().unwrap();
    buf
}

pub fn build_corrupt_wasm_bytes() -> Vec<u8> {
    let manifest = r#"
name = "corrupt-wasm-mod"
version = "1.0.0"
publisher = "adversary"
worldvm = "1"
abi = "1.0"
"#;
    // Corrupt magic header: \x00XYZ instead of \x00asm
    let corrupt_wasm = b"\x00XYZ\x01\x00\x00\x00";
    WorldModBuilder::new(manifest, corrupt_wasm.to_vec()).build().unwrap()
}

pub fn build_tampered_signature_package() -> Vec<u8> {
    let manifest = r#"
name = "tampered-sig-mod"
version = "1.0.0"
publisher = "adversary"
worldvm = "1"
abi = "1.0"
"#;
    let wasm = b"\x00asm\x01\x00\x00\x00";
    let (sk, _) = generate_keypair();
    let sig = sign_content(&sk, "original_content_hash_1234567890abcdef1234567890abcdef12345678");

    // Package with different actual content than signed content hash
    WorldModBuilder::new(manifest, wasm.to_vec())
        .with_signature(sig)
        .build()
        .unwrap()
}
