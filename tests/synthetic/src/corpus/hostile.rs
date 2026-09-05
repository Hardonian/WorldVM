//! Hostile Module Corpus: Loops, memory bombs, escalation, forged handles.

use worldvm_package::{WorldModBuilder, WorldModPackage};

fn compile_wat(wat: &str) -> Vec<u8> {
    wat::parse_str(wat).expect("Valid WAT")
}

pub fn build_infinite_loop_module() -> WorldModPackage {
    let wat = r#"(module
  (memory (export "memory") 1)
  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (loop (br 0))
    (i32.const 0))
)"#;
    let manifest = r#"
name = "hostile-loop-mod"
version = "1.0.0"
publisher = "adversary"
worldvm = "1"
abi = "1.0"
[resources]
memory_mb = 16
fuel = 50000
max_execution_ms = 2
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

pub fn build_memory_bomb_module() -> WorldModPackage {
    let wat = r#"(module
  (memory (export "memory") 1)
  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    ;; Attempt to grow linear memory by 10,000 pages (640 MB)
    (memory.grow (i32.const 10000))
    drop
    (i32.const 0))
)"#;
    let manifest = r#"
name = "hostile-memory-mod"
version = "1.0.0"
publisher = "adversary"
worldvm = "1"
abi = "1.0"
[resources]
memory_mb = 16
fuel = 200000
max_execution_ms = 5
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

pub fn build_host_call_storm_module() -> WorldModPackage {
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "player.read_position")
  (data (i32.const 32) "{\"player_id\":\"p1\"}")

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (local $i i32)
    (local.set $i (i32.const 0))
    (loop $l
      (call $host_call (i32.const 0) (i32.const 20) (i32.const 32) (i32.const 18) (i32.const 200) (i32.const 204))
      drop
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $l (i32.lt_s (local.get $i) (i32.const 10000)))
    )
    (i32.const 0))
)"#;
    let manifest = r#"
name = "hostile-flood-mod"
version = "1.0.0"
publisher = "adversary"
worldvm = "1"
abi = "1.0"
[permissions]
request = ["player.read_position"]
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

pub fn build_capability_escalation_module() -> WorldModPackage {
    // Module requests 'inventory.read', but attempts 'inventory.grant'
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "inventory.grant")
  (data (i32.const 32) "{\"player_id\":\"p1\",\"item_id\":\"gold\",\"quantity\":100000}")

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (call $host_call (i32.const 0) (i32.const 15) (i32.const 32) (i32.const 56) (i32.const 200) (i32.const 204)))
)"#;
    let manifest = r#"
name = "hostile-escalation-mod"
version = "1.0.0"
publisher = "adversary"
worldvm = "1"
abi = "1.0"
[permissions]
request = ["inventory.read"]
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

pub fn build_string_confusion_module() -> WorldModPackage {
    // Attempts capability traversal: "world.set_gravity/../admin"
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "world.set_gravity/../admin")

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (call $host_call (i32.const 0) (i32.const 26) (i32.const 0) (i32.const 0) (i32.const 200) (i32.const 204)))
)"#;
    let manifest = r#"
name = "hostile-confusion-mod"
version = "1.0.0"
publisher = "adversary"
worldvm = "1"
abi = "1.0"
[permissions]
request = ["world.set_gravity"]
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}
