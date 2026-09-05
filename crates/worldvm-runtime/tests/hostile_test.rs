use std::sync::Arc;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_core::WorldVmError;
use worldvm_package::{WorldModBuilder, WorldModPackage};
use worldvm_runtime::WorldVmRuntime;
use worldvm_simulator::MockGameHost;

/// WAT boilerplate for valid WorldVM guest exports.
fn make_wat_module(body: &str) -> Vec<u8> {
    let wat = format!(
        r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)

  (func (export "worldvm_guest_alloc") (param i32) (result i32)
    (i32.const 1024))

  (func (export "worldvm_guest_free") (param i32 i32))

  (func (export "worldvm_get_abi_version") (result i32)
    (i32.const 256))

  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    {body}
    (i32.const 0))
)"#
    );
    wat::parse_str(&wat).expect("Valid WAT")
}

#[test]
fn test_infinite_loop_caught_by_fuel_metering() {
    // WAT with an infinite loop: (loop (br 0))
    let wasm = make_wat_module("(loop (br 0))");

    let manifest = r#"
name = "infinite-loop-mod"
version = "1.0.0"
publisher = "hostile_author"
worldvm = "1"
abi = "1.0"

[resources]
memory_mb = 16
fuel = 50000
max_execution_ms = 5
"#;

    let package_bytes = WorldModBuilder::new(manifest, wasm).build().unwrap();
    let pkg = WorldModPackage::from_bytes(&package_bytes).unwrap();

    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    runtime.load_module(pkg).unwrap();

    let res = runtime.emit_event("infinite-loop-mod", "tick", b"{}");
    assert!(
        matches!(res, Err(WorldVmError::OutOfFuel { .. })),
        "Infinite loop must be cleanly trapped as OutOfFuel, got: {:?}",
        res
    );
}

#[test]
fn test_wasm_unreachable_trap_isolated() {
    // WAT with explicit unreachable trap instruction
    let wasm = make_wat_module("(unreachable)");

    let manifest = r#"
name = "trap-mod"
version = "1.0.0"
publisher = "hostile_author"
worldvm = "1"
abi = "1.0"
"#;

    let package_bytes = WorldModBuilder::new(manifest, wasm).build().unwrap();
    let pkg = WorldModPackage::from_bytes(&package_bytes).unwrap();

    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    runtime.load_module(pkg).unwrap();

    let res = runtime.emit_event("trap-mod", "tick", b"{}");
    assert!(
        matches!(res, Err(WorldVmError::ModuleTrap { .. })),
        "Explicit trap must be caught without host panic, got: {:?}",
        res
    );
}

#[test]
fn test_circuit_breaker_trips_after_consecutive_failures() {
    let wasm = make_wat_module("(unreachable)");

    let manifest = r#"
name = "crash-loop-mod"
version = "1.0.0"
publisher = "hostile_author"
worldvm = "1"
abi = "1.0"
"#;

    let package_bytes = WorldModBuilder::new(manifest, wasm).build().unwrap();
    let pkg = WorldModPackage::from_bytes(&package_bytes).unwrap();

    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    runtime.load_module(pkg).unwrap();

    // 3 consecutive traps
    let _ = runtime.emit_event("crash-loop-mod", "tick", b"{}");
    let _ = runtime.emit_event("crash-loop-mod", "tick", b"{}");
    let _ = runtime.emit_event("crash-loop-mod", "tick", b"{}");

    // 4th invocation must be disabled by circuit breaker
    let fourth = runtime.emit_event("crash-loop-mod", "tick", b"{}");
    assert!(
        matches!(fourth, Err(WorldVmError::CircuitBreakerTripped { .. })),
        "Repeated failures must trip circuit breaker, got: {:?}",
        fourth
    );
}

#[test]
fn test_forbidden_imports_rejected_at_load_time() {
    let wat = r#"(module
  (import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))
  (memory 1)
  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 0))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32) (i32.const 0))
)"#;
    let wasm = wat::parse_str(wat).unwrap();

    let manifest = r#"
name = "wasi-escape-mod"
version = "1.0.0"
publisher = "hostile_author"
worldvm = "1"
abi = "1.0"
"#;

    let package_bytes = WorldModBuilder::new(manifest, wasm).build().unwrap();
    let pkg = WorldModPackage::from_bytes(&package_bytes).unwrap();

    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    let res = runtime.load_module(pkg);
    assert!(
        matches!(res, Err(WorldVmError::InvalidPackage { ref reason }) if reason.contains("forbidden or undeclared namespace")),
        "Module importing external namespace must be rejected, got: {:?}",
        res
    );
}

#[test]
fn test_unauthorized_host_call_denied_in_sandbox() {
    // WAT calls host with capability "network.http" (which is denied by default arcade contract)
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)

  (data (i32.const 0) "network.http")

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))

  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    ;; Call worldvm_host_call("network.http", len 12, in_ptr 0, in_len 0, out_ptr 200, out_len 204)
    (call $host_call (i32.const 0) (i32.const 12) (i32.const 0) (i32.const 0) (i32.const 200) (i32.const 204))
    drop
    (i32.const 0))
)"#;
    let wasm = wat::parse_str(wat).unwrap();

    let manifest = r#"
name = "malicious-network-mod"
version = "1.0.0"
publisher = "hostile_author"
worldvm = "1"
abi = "1.0"

[permissions]
request = ["network.http"]
"#;

    let package_bytes = WorldModBuilder::new(manifest, wasm).build().unwrap();
    let pkg = WorldModPackage::from_bytes(&package_bytes).unwrap();

    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");
    let mut runtime = WorldVmRuntime::new(contract, host.clone(), false).unwrap();

    runtime.load_module(pkg).unwrap();

    let res = runtime.emit_event("malicious-network-mod", "test", b"{}");
    assert!(res.is_ok(), "Event dispatch returned cleanly");

    // Verify host was NOT called for network.http
    let calls = host.get_capability_history();
    assert!(calls.is_empty(), "Denied capability must NEVER reach the host");

    // Verify denial was recorded
    let denial = runtime.get_last_denial("malicious-network-mod");
    assert!(denial.is_some(), "Denial reason must be recorded in runtime");
    assert!(denial.unwrap().contains("denied by host contract"));
}
