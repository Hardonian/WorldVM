use std::io::Cursor;
use worldvm_core::WorldVmError;
use worldvm_package::{WorldModBuilder, WorldModPackage};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;
use std::io::Write;

#[test]
fn test_package_roundtrip() {
    let manifest = r#"
name = "test-mod"
version = "1.0.0"
publisher = "author_1"
worldvm = "1"
abi = "1.0"

[resources]
memory_mb = 16
fuel = 100000
max_execution_ms = 2

[permissions]
request = ["world.set_gravity"]

[events]
subscribe = ["round_start"]
"#;

    let wasm = b"\x00asm\x01\x00\x00\x00";

    let builder = WorldModBuilder::new(manifest, wasm.to_vec())
        .add_asset("assets/config.json", b"{\"speed\": 10}".to_vec());

    let package_bytes = builder.build().expect("Package build succeeded");
    let pkg = WorldModPackage::from_bytes(&package_bytes).expect("Package unpack succeeded");

    assert_eq!(pkg.manifest.name, "test-mod");
    assert_eq!(pkg.manifest.version, "1.0.0");
    assert_eq!(pkg.wasm_bytes, wasm);
    assert_eq!(pkg.assets.len(), 1);
    assert!(pkg.assets.contains_key("assets/config.json"));
}

#[test]
fn test_zip_slip_traversal_rejected() {
    let mut buf = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buf));
    let options = SimpleFileOptions::default();

    writer.start_file("../../etc/passwd", options).unwrap();
    writer.write_all(b"malicious").unwrap();
    writer.finish().unwrap();

    let res = WorldModPackage::from_bytes(&buf);
    assert!(matches!(res, Err(WorldVmError::InvalidPackage { ref reason }) if reason.contains("path traversal")));
}

#[test]
fn test_native_executable_rejected() {
    let mut buf = Vec::new();
    let mut writer = ZipWriter::new(Cursor::new(&mut buf));
    let options = SimpleFileOptions::default();

    writer.start_file("malicious.dll", options).unwrap();
    writer.write_all(b"MZ\x90\x00").unwrap();
    writer.finish().unwrap();

    let res = WorldModPackage::from_bytes(&buf);
    assert!(matches!(res, Err(WorldVmError::InvalidPackage { ref reason }) if reason.contains("prohibited native executable")));
}

#[test]
fn test_abi_mismatch_rejected() {
    let manifest = r#"
name = "bad-abi"
version = "1.0.0"
publisher = "author"
worldvm = "1"
abi = "99.0"
"#;
    let wasm = b"\x00asm\x01\x00\x00\x00";
    let builder = WorldModBuilder::new(manifest, wasm.to_vec());
    let bytes = builder.build().unwrap();

    let res = WorldModPackage::from_bytes(&bytes);
    assert!(matches!(res, Err(WorldVmError::AbiMismatch { .. })));
}
