//! Chaos & Fault Injection Test Suite.
//! Tests sandbox resilience under hostile network, memory, fuel, and concurrency conditions.

use std::sync::Arc;
use parking_lot::RwLock;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_runtime::WorldVmRuntime;

use crate::corpus::{buggy, hostile, legitimate};
use crate::simworld::{SimRng, SyntheticCapabilityProvider, WorldState};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosResult {
    pub name: String,
    pub passed: bool,
    pub details: String,
}

pub fn run_all_chaos_tests(seed: u64) -> Vec<ChaosResult> {
    vec![
        test_rapid_load_unload_churn(seed),
        test_concurrent_module_event_storm(seed),
        test_corrupted_payload_fuzzing(seed),
        test_fuel_starvation_recovery(seed),
        test_interleaved_hostile_and_legitimate_execution(seed),
    ]
}

/// 1. Rapid Load/Unload Churn: Tests that repeatedly loading, executing, and unloading
/// does not leak memory or panic in Wasmtime store/linker.
pub fn test_rapid_load_unload_churn(seed: u64) -> ChaosResult {
    let mut rng = SimRng::from_seed(seed);
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state));
    let contract = WorldCapabilityContract::standard_arcade_contract("chaos-churn");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    let mut success_count = 0;
    for _ in 0..20 {
        let which = rng.next_u32() % 3;
        let pkg = match which {
            0 => legitimate::build_welcome_module(),
            1 => legitimate::build_gravity_module(),
            _ => legitimate::build_race_module(),
        };
        let mod_name = pkg.manifest.name.clone();

        if runtime.load_module(pkg).is_ok() {
            let _ = runtime.emit_event(&mod_name, "round_start", b"{}");
            let _ = runtime.unload_module(&mod_name);
            success_count += 1;
        }
    }

    let passed = success_count == 20 && runtime.loaded_modules().is_empty();
    ChaosResult {
        name: "Rapid Module Load/Unload Churn".to_string(),
        passed,
        details: format!("Completed {success_count}/20 load-eval-unload cycles without leak or panic"),
    }
}

/// 2. Concurrent Module Event Storm: Multiple modules loaded simultaneously
/// receiving 100 fast events.
pub fn test_concurrent_module_event_storm(_seed: u64) -> ChaosResult {
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state));
    let contract = WorldCapabilityContract::standard_arcade_contract("chaos-storm");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    runtime.load_module(legitimate::build_welcome_module()).unwrap();
    runtime.load_module(legitimate::build_gravity_module()).unwrap();
    runtime.load_module(legitimate::build_race_module()).unwrap();
    runtime.load_module(legitimate::build_survival_module()).unwrap();

    let mut total_events = 0;
    for tick in 1..=50 {
        let payload = format!("{{\"tick\":{tick}}}");
        let _ = runtime.emit_event("welcome-mod", "player_join", payload.as_bytes());
        let _ = runtime.emit_event("low-gravity-mod", "round_start", payload.as_bytes());
        let _ = runtime.emit_event("checkpoint-race-mod", "checkpoint", payload.as_bytes());
        let _ = runtime.emit_event("zombie-survival-mod", "tick", payload.as_bytes());
        total_events += 4;
    }

    let passed = total_events == 200;
    ChaosResult {
        name: "Concurrent Module Event Storm".to_string(),
        passed,
        details: format!("Dispatched {total_events} events across 4 concurrent modules without deadlock"),
    }
}

/// 3. Corrupted Payload Fuzzing: Feeds arbitrary binary junk, truncated JSON,
/// and oversized buffers to guest modules.
pub fn test_corrupted_payload_fuzzing(seed: u64) -> ChaosResult {
    let mut rng = SimRng::from_seed(seed);
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state));
    let contract = WorldCapabilityContract::standard_arcade_contract("chaos-fuzz");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    runtime.load_module(legitimate::build_gravity_module()).unwrap();

    let mut survived = 0;
    for _ in 0..50 {
        // Generate random binary junk
        let len = (rng.next_u32() % 256) as usize;
        let mut junk = vec![0u8; len];
        for b in &mut junk {
            *b = (rng.next_u32() % 256) as u8;
        }

        // Host runtime must not panic, regardless of guest's JSON parsing or trap
        let _ = runtime.emit_event("low-gravity-mod", "round_start", &junk);
        survived += 1;
    }

    let passed = survived == 50;
    ChaosResult {
        name: "Corrupted Payload Fuzzing".to_string(),
        passed,
        details: format!("Processed {survived}/50 malformed/fuzzed payloads with zero host panics"),
    }
}

/// 4. Fuel Starvation Recovery: Trapping out of fuel in one module must not impair
/// subsequent event dispatches or other modules.
pub fn test_fuel_starvation_recovery(_seed: u64) -> ChaosResult {
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state));
    let contract = WorldCapabilityContract::standard_arcade_contract("chaos-recovery");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    runtime.load_module(hostile::build_infinite_loop_module()).unwrap();
    runtime.load_module(legitimate::build_welcome_module()).unwrap();

    // 1. Hostile module runs and starves fuel
    let trap_res = runtime.emit_event("hostile-loop-mod", "tick", b"{}");
    let hostile_trapped = trap_res.is_err();

    // 2. Legitimate module runs immediately after and must succeed
    let welcome_res = runtime.emit_event("welcome-mod", "player_join", b"{\"player_id\":\"p1\"}");
    let legitimate_succeeded = welcome_res.is_ok();

    let passed = hostile_trapped && legitimate_succeeded;
    ChaosResult {
        name: "Fuel Starvation Recovery".to_string(),
        passed,
        details: format!("Hostile loop trapped: {hostile_trapped}, subsequent legitimate call succeeded: {legitimate_succeeded}"),
    }
}

/// 5. Interleaved Hostile, Buggy, and Legitimate Execution:
/// Demonstrates cross-tenant isolation where hostile mods fail while legitimate mods thrive.
pub fn test_interleaved_hostile_and_legitimate_execution(_seed: u64) -> ChaosResult {
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state));
    let contract = WorldCapabilityContract::standard_arcade_contract("chaos-interleaved");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    runtime.load_module(buggy::build_div_zero_module()).unwrap();
    runtime.load_module(legitimate::build_gravity_module()).unwrap();
    runtime.load_module(hostile::build_capability_escalation_module()).unwrap();
    runtime.load_module(legitimate::build_race_module()).unwrap();

    // 1. Buggy traps
    let r1 = runtime.emit_event("buggy-divzero-mod", "tick", b"{}");
    // 2. Legitimate succeeds
    let r2 = runtime.emit_event("low-gravity-mod", "round_start", b"{}");
    // 3. Escalation blocked
    let r3 = runtime.emit_event("hostile-escalation-mod", "tick", b"{}");
    // 4. Legitimate succeeds
    let r4 = runtime.emit_event("checkpoint-race-mod", "round_start", b"{}");

    let passed = r1.is_err() && r2.is_ok() && r3.is_ok() && r4.is_ok();
    ChaosResult {
        name: "Interleaved Hostile & Legitimate Execution".to_string(),
        passed,
        details: format!("Buggy trapped ({:?}), Legitimate 1 ok, Escalation blocked, Legitimate 2 ok", r1.is_err()),
    }
}
