//! Buggy Module Corpus: Traps, div-by-zero, out-of-bounds, stack recursion.

use worldvm_package::{WorldModBuilder, WorldModPackage};

fn compile_wat(wat: &str) -> Vec<u8> {
    wat::parse_str(wat).expect("Valid WAT")
}

pub fn build_trap_module() -> WorldModPackage {
    let wat = r#"(module
  (memory (export "memory") 1)
  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (unreachable))
)"#;
    let manifest = r#"
name = "buggy-trap-mod"
version = "1.0.0"
publisher = "buggy_creator"
worldvm = "1"
abi = "1.0"
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

pub fn build_div_zero_module() -> WorldModPackage {
    let wat = r#"(module
  (memory (export "memory") 1)
  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    ;; 100 / 0
    (i32.div_s (i32.const 100) (i32.const 0)))
)"#;
    let manifest = r#"
name = "buggy-div-zero-mod"
version = "1.0.0"
publisher = "buggy_creator"
worldvm = "1"
abi = "1.0"
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

pub fn build_out_of_bounds_module() -> WorldModPackage {
    let wat = r#"(module
  (memory (export "memory") 1)
  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    ;; Load from out of bounds memory address 10,000,000
    (i32.load (i32.const 10000000)))
)"#;
    let manifest = r#"
name = "buggy-oob-mod"
version = "1.0.0"
publisher = "buggy_creator"
worldvm = "1"
abi = "1.0"
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

pub fn build_recursion_module() -> WorldModPackage {
    let wat = r#"(module
  (memory (export "memory") 1)
  (func $recurse (param i32) (result i32)
    (call $recurse (i32.add (local.get 0) (i32.const 1))))
  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (call $recurse (i32.const 0)))
)"#;
    let manifest = r#"
name = "buggy-recursion-mod"
version = "1.0.0"
publisher = "buggy_creator"
worldvm = "1"
abi = "1.0"
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}
