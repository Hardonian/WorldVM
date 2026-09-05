# WorldVM

> **Turn any game into a creator platform**  
> *Run creator-built gameplay safely inside Unity, Unreal, Godot and custom engines.*  
> *Sandboxed code. Capability permissions. WASM. One runtime across games.*

[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![WASM Runtime: Wasmtime 48](https://img.shields.io/badge/wasm-Wasmtime%2048-orange.svg)](https://wasmtime.dev)
[![Synthetic Validation: 25/25 PASS](https://img.shields.io/badge/synthetic%20tests-25%2F25%20PASS%20(100%25)-brightgreen.svg)](docs/SYNTHETIC_TEST_REPORT.md)
[![Frame Budget: 27µs p99](https://img.shields.io/badge/p99%20latency-27µs%20%40%2060Hz-success.svg)](docs/SYNTHETIC_TEST_REPORT.md)

![WorldVM Hero Banner](assets/hero-banner.jpg)

---

## Why WorldVM?

Game studios want community-driven longevity: **mods, creator-built game modes, live game rules, custom quests, and challenge runs**. But traditional approaches are fraught with peril:

- **Lua / Python scripting** exposes games to global namespace pollution, unpredictable garbage collection pauses, and hard-to-diagnose memory leaks.
- **Native DLLs / C++ mods** are an infosec catastrophe: arbitrary disk access, malware injection risks, and game-crashing memory corruptions.
- **Engine-specific modding systems** force studios to maintain fragmented tooling for Unity, Unreal, and custom internal engines.

**WorldVM provides a universal WebAssembly sandbox and capability-based security model.** Untrusted creator code runs in a microsecond-fast, deterministic WASM sandbox with zero ambient authority.

---

## Key Features

- 🛡️ **Zero Ambient Authority**: Modules cannot touch disk, network, or OS APIs. WASI is explicitly denied.
- ⚡ **Microsecond Frame Budget**: Average tick overhead of **~16 microseconds** at 60 Hz/120 Hz. Zero frame stutter or hitching.
- ⛽ **Fuel Metering & Epoch Interruption**: Infinite loops (`while(true) {}`) trap cleanly in <1ms without freezing the game engine.
- 📜 **Fine-Grained Capability Contracts**: Studios define exact permissions (`world.spawn`, `player.grant_xp`, `ui.notify`) with per-tick rate limits in clean YAML.
- 📦 **`.worldmod` Self-Contained Packages**: Signed ZIP archives with Ed25519 tamper detection, Zip Slip defense, and decompression bomb protection.
- 🌐 **Multi-Engine C ABI**: Native integration for **Unity (UPM)**, **Godot 4.x (GDExtension)**, **Unreal Engine 5**, and custom C++/Rust engines.
- 🔄 **Deterministic Multiplayer**: Lockstep state verification with cryptographic SHA-256 state hashing and desync divergence detection.

---

## Architecture & Security Model

![WASM Game Sandbox Security Architecture](assets/security-architecture.jpg)

```text
+-------------------------------------------------------------------------+
|                              GAME ENGINE                                |
|        (Unity / Unreal Engine 5 / Godot 4 / Custom C++ / Rust)          |
+-------------------------------------------------------------------------+
       |                                                    ^
       | Events (round_start, tick, player_join)            | Host Calls
       v                                                    | (spawn, notify)
+-------------------------------------------------------------------------+
|                             WORLDVM HOST                                |
|                                                                         |
|  +-----------------------+     +-------------------------------------+  |
|  |   Capability Enforcer |     |      Execution Engine (Wasmtime 48) |  |
|  | - Location (Clt/Srv)  |     | - Instruction Fuel Metering         |  |
|  | - Rate Limits/Tick    |     | - Epoch Interruption Deadlines      |  |
|  | - Category ACLs       |     | - Linear Memory Limit (8MB-64MB)    |  |
|  +-----------------------+     +-------------------------------------+  |
|              ^                                    |                     |
|              | check_call                         | instantiates        |
|  +-----------------------+                        v                     |
|  |   Capability Contract |             +---------------------+          |
|  |   (game_policy.yaml)  |             | GUEST WASM SANDBOX  |          |
|  +-----------------------+             | (.worldmod package) |          |
+----------------------------------------+---------------------+----------+
```

---

## 5-Minute Quickstart

### 1. Install CLI

```bash
cargo install --path crates/worldvm-cli
worldvm doctor
```

### 2. Create a Mod

```bash
worldvm init my-gravity-mod
cd my-gravity-mod
```

### 3. Write Gameplay Logic (`src/lib.rs`)

```rust
use worldvm_sdk::prelude::*;

#[derive(Default)]
pub struct LowGravityMod;

impl WorldVmEntrypoint for LowGravityMod {
    fn on_event(&mut self, event_name: &str, _payload: &[u8]) -> Result<(), String> {
        if event_name == "round_start" {
            // Set moon gravity (2.40 m/s^2)
            world::set_gravity(2.40).map_err(|e| e.to_string())?;
            // Send toast message to HUD
            ui::notify_player("all", "Lunar Gravity Activated! (2.40 m/s^2)").map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

export_entrypoint!(LowGravityMod);
```

### 4. Build, Package & Verify

```bash
worldvm build
worldvm package --out dist/my-gravity-mod.worldmod
worldvm inspect dist/my-gravity-mod.worldmod
```

### 5. Run in Reference Game

```bash
cargo run -p reference-game
```

---

## Empirical Validation & Evidence Matrix

WorldVM rejects fake pass rates. All metrics are measured by the **Autonomous Synthetic Test Lab** (`tests/synthetic`):

```text
=======================================================
  WORLDVM SYNTHETIC TEST LAB — FULL STACK VALIDATION  
=======================================================
Profile : ci
Seed    : 42

RESULTS: 25/25 tests passed (100.0%) in 1105 ms
```

### Red Team Security Matrix (S001 – S015)

| ID | Attack Scenario | Status | Measured Result |
| --- | --- | --- | --- |
| **S001** | Infinite Loop Fuel Depletion | **PASS** | `OutOfFuel { fuel_limit: 50000, consumed: 50000 }` (Trapped cleanly) |
| **S002** | Linear Memory Bomb Containment | **PASS** | Bounded at 16MB page allocation limit |
| **S003** | Capability Escalation Defense | **PASS** | Blocked (`Permission denied: Capability not exposed`) |
| **S004** | Forged Handle Safety | **PASS** | Blocked (`HostError: Player not found`) |
| **S005** | Zip Slip Path Traversal | **PASS** | Rejected (`Malicious path traversal detected: ../../../root/shadow`) |
| **S006** | Tampered Signature Rejection | **PASS** | Rejected (`Content hash mismatch`) |
| **S007** | Cross-Module State Isolation | **PASS** | Modules cannot inspect peer storage keys |
| **S008** | Cross-Game Policy Isolation | **PASS** | Enforced by unique Game ID boundaries |
| **S009** | Network SSRF Protection | **PASS** | Blocked (`SSRF Attempt blocked: internal address forbidden`) |
| **S010** | Host Call Storm Quota | **PASS** | Capped at 32 calls/tick rate limit |
| **S011** | Event Storm Resilience | **PASS** | Handled 50 consecutive trapped events without host hang |
| **S012** | Exclusive Capability ModSet Conflict | **PASS** | Incompatible mods rejected on load |
| **S013** | Circuit Breaker Automatic Trip | **PASS** | Disabled after 3 consecutive unhandled failures |
| **S014** | Economy Integer Integrity | **PASS** | Exact 64-bit integer arithmetic without float drift |
| **S015** | Module Unload Cleanliness | **PASS** | Store and memory dropped with zero resource leaks |

### Frame Budget Benchmarks

| Target Cadence | Target Frame Time | Measured Avg Latency | P99 Latency | Budget Spikes |
| --- | --- | --- | --- | --- |
| **30 Hz** | 33,333 µs | **16.4 µs** | **94 µs** | 0 |
| **60 Hz** | 16,667 µs | **15.6 µs** | **27 µs** | 0 |
| **120 Hz** | 8,333 µs | **15.4 µs** | **24 µs** | 0 |

---

## Engine Evidence Matrix

We enforce an honest, evidence-based classification for all engine integrations:

| Engine / Target | Evidence Class | Validation Target | Status | Notes |
| --- | --- | --- | --- | --- |
| **Native Rust Engine** | `SIMULATION_VERIFIED` | `reference-game` (Neon Arena) | **PASS** | Multi-mod physics, dynamic gravity, zombie spawns, hostile containment. |
| **C ABI Host Library** | `BUILD_VERIFIED` | `crates/worldvm-c-api/examples/main.c` | **PASS** | DLL compiled and executed with `clang`/`lld`. |
| **Godot 4.x GDExtension** | `BUILD_VERIFIED` | `sdk/godot/bin/worldvm.gdextension` | **PASS** | GDExtension configuration and GDScript wrapper verified. |
| **Unity Engine (UPM)** | `UNIT_VERIFIED` | `sdk/unity/Runtime/WorldVM.cs` | **PASS** | C# P/Invoke bindings, struct layouts, and delegate marshaling verified. |
| **Unreal Engine 5** | `INTEGRATION_READY_UNVERIFIED` | `sdk/unreal/WorldVM.uplugin` | **READY** | C++ `UWorldSubsystem` headers prepared for headless UBT cluster build. |

*See [docs/VALIDATION_GAPS.md](docs/VALIDATION_GAPS.md) for detailed descriptions and the roadmap to promote all targets to `SIMULATION_VERIFIED`.*

---

## Documentation Index

- 📘 [Quickstart Guide](docs/QUICKSTART.md)
- 🏗️ [Architecture Specification](docs/ARCHITECTURE.md)
- 🔒 [Security Guarantees](docs/SECURITY.md)
- 🎯 [Threat Model](docs/THREAT_MODEL.md)
- 💰 [Execution Economics & Metering](docs/EXECUTION_ECONOMICS.md)
- 🛡️ [Capability System](docs/CAPABILITY_SYSTEM.md)
- 📦 [`.worldmod` Package Specification](docs/MOD_PACKAGE_SPEC.md)
- 📊 [Synthetic Validation Report](docs/SYNTHETIC_TEST_REPORT.md)
- 🛣️ [Validation Gaps & Roadmap](docs/VALIDATION_GAPS.md)

---

## License

WorldVM is licensed under the [Apache License, Version 2.0](LICENSE).
