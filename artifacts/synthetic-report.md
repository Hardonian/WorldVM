# WorldVM Synthetic Validation Report

**Version**: 1.0.0  
**Timestamp**: 2026-09-05T03:45:00Z  
**Profile**: `ci`  

## Executive Summary

* **Total Tests Executed**: 28
* **Passed**: 28
* **Failed**: 0
* **Pass Rate**: 100.0%
* **Duration**: 1116 ms

## Red Team Security Scenarios (S001 – S015)

| ID | Attack Scenario | Status | Details |
|---|---|---|---|
| S001 | Infinite Loop Fuel Depletion | PASS | Result: Err(OutOfFuel { fuel_limit: 50000, consumed: 50000 }) |
| S002 | Linear Memory Growth Containment | PASS | Result: Ok(ExecutionMetrics { invocations: 1, fuel_consumed: 10, execution_time_us: 603, host_calls: 0, memory_high_water_mark_bytes: 0, errors_encountered: 0 }) |
| S003 | Capability Escalation Defense | PASS | Escalation reached host: false, Denial: Some("Capability 'inventory.grant' denied: Permission denied for capability 'inventory.grant': Capability is not exposed by host contract") |
| S004 | Forged Handle Safety | PASS | Forged handle response: Err(HostError { message: "Player 'forged_player_9999999' not found" }) |
| S005 | Zip Slip Path Traversal Defense | PASS | Unpack result: Err(InvalidPackage { reason: "Malicious path traversal detected in package: ../../../root/shadow" }) |
| S006 | Tampered Signature Rejection | PASS | Verify result: Err(InvalidSignature { reason: "Content hash mismatch: signature hash is original_content_hash_1234567890abcdef1234567890abcdef12345678, calculated hash is 954b2521a646c88cce02ac40be2fb7c68dff10101457d910c61e4e5a16ca2a1b" }) |
| S007 | Cross-Module State Isolation | PASS | Mod B read from Mod A's key: '' |
| S008 | Cross-Game Policy Isolation | PASS | Contracts isolated by GameId: game_alpha vs game_beta |
| S009 | Network SSRF Protection | PASS | SSRF call result: Err(PermissionDenied { capability: "network.http", reason: "SSRF Attempt blocked: internal address forbidden" }) |
| S010 | Host Call Storm Quota Enforcement | PASS | Audit host call count: 32 (Target <= 32), Invocation: Ok(ExecutionMetrics { invocations: 1, fuel_consumed: 150010, execution_time_us: 20973, host_calls: 32, memory_high_water_mark_bytes: 0, errors_encountered: 9968 }) |
| S011 | Event Storm Resilience | PASS | Handled 50 consecutive trapped events without host crash |
| S012 | Exclusive Capability ModSet Conflict | PASS | Conflict detected: true |
| S013 | Circuit Breaker Automatic Disabling | PASS | Fourth invocation: Err(CircuitBreakerTripped { module_id: "buggy-trap-mod" }) |
| S014 | Economy State Exact Integer Integrity | PASS | XP updated from 50 to expected 150: actual 150 |
| S015 | Module Unload Lifecycle Cleanliness | PASS | Unloaded: true, Post-unload execution: Err(ModuleNotLoaded { module_id: "buggy-trap-mod" }) |
| S016 | Adaptive Behavioral Anomaly Detection | PASS | Score: 0.49, Level: Elevated, Tarpit Delay: 500us |
| S017 | Tarpit Defense & Automated Signature Generation | PASS | Level: Critical, Quarantined: true, Signatures Generated: 1 |
| S018 | Marketplace Revenue Splits & Signed Compute Receipts | PASS | Split Verified: true (Creator: $17.50, Platform: $2.50), Receipt Sig: true |

## Multiplayer Determinism & Desync

* **Deterministic Lockstep Hash Consistency**: PASS (Identical state hash across runs)
* **Desync Detection on Divergent Inputs**: PASS (Divergence cleanly detected)

## Game Simulation Scenarios

* **Neon Racer**: PASS (Laps: 3, Ticks: 300, XP: 10000, Winner: `p4`)
* **Arena Zombie Survival**: PASS (Spawned: 10, Damage Events: 32, Survivors: 4/4, Score: 1600)

## Frame Budget Benchmarks

| Target Cadence | Target Frame Time | Avg Execution Time | P99 Latency | Budget Exceeded |
|---|---|---|---|---|
| 30 Hz | 33333 us | 19.4 us | 110 us | 0 |
| 60 Hz | 16667 us | 19.6 us | 83 us | 0 |
| 120 Hz | 8333 us | 18.6 us | 31 us | 0 |

## Chaos & Fault Injection

| Scenario | Status | Details |
|---|---|---|
| Rapid Module Load/Unload Churn | PASS | Completed 20/20 load-eval-unload cycles without leak or panic |
| Concurrent Module Event Storm | PASS | Dispatched 200 events across 4 concurrent modules without deadlock |
| Corrupted Payload Fuzzing | PASS | Processed 50/50 malformed/fuzzed payloads with zero host panics |
| Fuel Starvation Recovery | PASS | Hostile loop trapped: true, subsequent legitimate call succeeded: true |
| Interleaved Hostile & Legitimate Execution | PASS | Buggy trapped (true), Legitimate 1 ok, Escalation blocked, Legitimate 2 ok |

## Engine Evidence Classification

| Engine | Evidence Class | Test Target | Status | Notes |
|---|---|---|---|---|
| Native Rust Engine | SIMULATION_VERIFIED | reference-game (Neon Arena) | PASS | End-to-end multi-mod integration verified with low-gravity, zombie-spawner, and malicious containment. |
| C ABI / Host Runtime | BUILD_VERIFIED | crates/worldvm-c-api/examples/main.c | PASS | DLL compilation, header linking, and C host instantiation verified with clang/lld. |
| Godot 4.x GDExtension | BUILD_VERIFIED | sdk/godot/bin/worldvm.gdextension | PASS | C ABI dynamic library wrapper generated and validated against Godot GDExtension ABI spec. |
| Unity Engine (UPM) | UNIT_VERIFIED | sdk/unity/Runtime/WorldVM.cs | PASS | C# P/Invoke bindings, struct layouts, and delegate marshaling validated. |
| Unreal Engine 5 | INTEGRATION_READY_UNVERIFIED | sdk/unreal/WorldVM.uplugin | READY | C++ UWorldSubsystem header structure ready; waiting on automated headless UE5 CI cluster runner. |

