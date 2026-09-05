//! Legitimate Module Corpus: TINY, SMALL, MEDIUM, HEAVY.

use worldvm_package::{WorldModBuilder, WorldModPackage};

fn compile_wat(wat: &str) -> Vec<u8> {
    wat::parse_str(wat).expect("Valid WAT")
}

/// TINY module: 1 event ('player_join'), 1 host notification call.
pub fn build_welcome_module() -> WorldModPackage {
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "ui.notify")
  (data (i32.const 32) "{\"player_id\":\"p1\",\"message\":\"Welcome!\",\"duration_seconds\":2.0}")

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))

  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (call $host_call (i32.const 0) (i32.const 9) (i32.const 32) (i32.const 60) (i32.const 200) (i32.const 204))
    drop
    (i32.const 0))
)"#;
    let manifest = r#"
name = "welcome-mod"
version = "1.0.0"
publisher = "synthetic_creator"
worldvm = "1"
abi = "1.0"

[resources]
memory_mb = 8
fuel = 100000
max_execution_ms = 2

[permissions]
request = ["ui.notify"]

[events]
subscribe = ["player_join"]
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

/// SMALL module: 2 events ('round_start', 'player_join'), sets gravity and notifies.
pub fn build_gravity_module() -> WorldModPackage {
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "world.set_gravity")
  (data (i32.const 32) "{\"gravity\": 2.40}")

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))

  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (call $host_call (i32.const 0) (i32.const 17) (i32.const 32) (i32.const 17) (i32.const 200) (i32.const 204))
    drop
    (i32.const 0))
)"#;
    let manifest = r#"
name = "low-gravity-mod"
version = "1.0.0"
publisher = "synthetic_creator"
worldvm = "1"
abi = "1.0"

[resources]
memory_mb = 16
fuel = 200000
max_execution_ms = 3

[permissions]
request = ["world.set_gravity", "ui.notify"]

[events]
subscribe = ["round_start", "player_join"]
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

/// MEDIUM module: Checkpoint race manager, grants XP on checkpoints.
pub fn build_race_module() -> WorldModPackage {
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "player.grant_xp")
  (data (i32.const 32) "{\"player_id\":\"p1\",\"amount\":250}")

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))

  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (call $host_call (i32.const 0) (i32.const 15) (i32.const 32) (i32.const 31) (i32.const 200) (i32.const 204))
    drop
    (i32.const 0))
)"#;
    let manifest = r#"
name = "checkpoint-race-mod"
version = "1.0.0"
publisher = "synthetic_creator"
worldvm = "1"
abi = "1.0"

[resources]
memory_mb = 16
fuel = 300000
max_execution_ms = 4

[permissions]
request = ["player.grant_xp", "ui.notify"]

[events]
subscribe = ["checkpoint", "round_start"]
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}

/// HEAVY module: Zombie wave spawner, spawns entities and manages state.
pub fn build_survival_module() -> WorldModPackage {
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "world.spawn")
  (data (i32.const 32) "{\"entity_type\":\"zombie\",\"x\":5.0,\"y\":0.0,\"z\":-10.0}")

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))

  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (call $host_call (i32.const 0) (i32.const 11) (i32.const 32) (i32.const 50) (i32.const 200) (i32.const 204))
    drop
    (i32.const 0))
)"#;
    let manifest = r#"
name = "zombie-survival-mod"
version = "1.0.0"
publisher = "synthetic_creator"
worldvm = "1"
abi = "1.0"

[resources]
memory_mb = 32
fuel = 500000
max_execution_ms = 5

[permissions]
request = ["world.spawn", "ui.notify", "player.apply_damage"]

[events]
subscribe = ["round_start", "tick"]
"#;
    let bytes = WorldModBuilder::new(manifest, compile_wat(wat)).build().unwrap();
    WorldModPackage::from_bytes(&bytes).unwrap()
}
