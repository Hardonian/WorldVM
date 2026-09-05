//! WorldVM Runtime Performance Benchmark Suite.
//! Measures module loading, JIT instantiation, event invocation, and host call dispatch overhead.

use std::sync::Arc;
use std::time::Instant;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_package::{WorldModBuilder, WorldModPackage};
use worldvm_runtime::WorldVmRuntime;
use worldvm_simulator::MockGameHost;

fn make_bench_module() -> Vec<u8> {
    let wat = r#"(module
  (import "worldvm_env" "worldvm_host_call" 
    (func $host_call (param i32 i32 i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)

  (func (export "worldvm_guest_alloc") (param i32) (result i32) (i32.const 1024))
  (func (export "worldvm_guest_free") (param i32 i32))
  (func (export "worldvm_get_abi_version") (result i32) (i32.const 256))

  (func (export "worldvm_handle_event") (param i32 i32 i32 i32) (result i32)
    (i32.const 0))
)"#;
    wat::parse_str(wat).expect("Valid WAT")
}

fn main() {
    println!("============================================================");
    println!("        WorldVM Runtime Performance Benchmark Suite         ");
    println!("============================================================");

    let wasm = make_bench_module();
    let manifest = r#"
name = "bench-mod"
version = "1.0.0"
publisher = "bench_author"
worldvm = "1"
abi = "1.0"

[resources]
memory_mb = 16
fuel = 1000000
max_execution_ms = 10
"#;

    let package_bytes = WorldModBuilder::new(manifest, wasm).build().unwrap();

    // 1. Benchmark Package Unpack & Validation
    let unpack_iters = 1_000;
    let t0 = Instant::now();
    for _ in 0..unpack_iters {
        let _ = WorldModPackage::from_bytes(&package_bytes).unwrap();
    }
    let unpack_total = t0.elapsed();
    let unpack_avg_us = unpack_total.as_micros() as f64 / unpack_iters as f64;
    println!(
        "  Package Unpack & SHA-256 Validation:  {:.2} µs / package ({:.0} pkgs/sec)",
        unpack_avg_us,
        1_000_000.0 / unpack_avg_us
    );

    // 2. Benchmark Module Load & Cranelift JIT Compilation
    let load_iters = 50;
    let pkg = WorldModPackage::from_bytes(&package_bytes).unwrap();
    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("bench");

    let t1 = Instant::now();
    for i in 0..load_iters {
        let mut runtime = WorldVmRuntime::new(contract.clone(), host.clone(), false).unwrap();
        let mut p = pkg.clone();
        p.manifest.name = format!("bench-mod-{}", i);
        runtime.load_module(p).unwrap();
    }
    let load_total = t1.elapsed();
    let load_avg_ms = load_total.as_secs_f64() * 1000.0 / load_iters as f64;
    println!(
        "  Module Load & Cranelift JIT Compile:  {:.2} ms / module",
        load_avg_ms
    );

    // 3. Benchmark Event Invocation Throughput (in sandbox with fuel metering)
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();
    runtime.load_module(pkg).unwrap();

    // Warmup
    for _ in 0..100 {
        let _ = runtime.emit_event("bench-mod", "tick", b"{}").unwrap();
    }

    let event_iters = 10_000;
    let t2 = Instant::now();
    let mut total_fuel = 0;
    for _ in 0..event_iters {
        let m = runtime.emit_event("bench-mod", "tick", b"{}").unwrap();
        total_fuel += m.fuel_consumed;
    }
    let event_total = t2.elapsed();
    let event_avg_us = event_total.as_micros() as f64 / event_iters as f64;
    let ops_per_sec = event_iters as f64 / event_total.as_secs_f64();

    println!(
        "  Sandboxed Event Dispatch (Metered):   {:.2} µs / event ({:.0} events/sec)",
        event_avg_us, ops_per_sec
    );
    println!(
        "  Average Fuel Overhead per Event:      {:.0} instructions",
        total_fuel as f64 / event_iters as f64
    );

    println!("============================================================\n");
}
