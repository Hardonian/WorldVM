# WorldVM Validation Gaps & Real-Engine Roadmap

## Overview

WorldVM enforces strict evidence classifications across all engine targets and runtime environments. Rather than claiming theoretical or paper compatibility, this document details what has been empirically verified and defines the explicit steps required to close each remaining validation gap.

---

## Evidence Classification Taxonomy

| Classification Level | Definition | Scope of Guarantee |
| --- | --- | --- |
| **`SIMULATION_VERIFIED`** | Fully executed against simulated and mock engine harnesses (e.g. `reference-game`, `SimWorld`, synthetic test lab). | Verified state mutations, deterministic replay, security containment, and cycle budgets. |
| **`BUILD_VERIFIED`** | Dynamic libraries compiled, linked, symbols exported, C headers parsed, and native binaries validated against ABI specifications. | Guarantees linker compatibility, struct layouts, calling conventions, and absence of symbol conflicts. |
| **`UNIT_VERIFIED`** | Language-specific bindings (C#, GDScript, C++) unit-tested for marshaling, memory lifetimes, and error code translations. | Guarantees API surface fidelity and binding correctness. |
| **`INTEGRATION_READY_UNVERIFIED`** | Plugin source code, manifests, and build scripts created and syntactically validated, but not yet executed inside a live closed-source engine editor/runner. | Ready for testing within licensed engine installations. |

---

## Current Status Matrix

| Target Engine / Component | Current Classification | Empirically Validated | Remaining Gap |
| --- | --- | --- | --- |
| **Native Rust Game Host** | `SIMULATION_VERIFIED` | Full multi-mod game loop, 60 Hz physics, dynamic gravity, entity spawning, security containment (`Neon Arena`). | None for standalone Rust runtimes. |
| **C ABI Shared Library (`worldvm_c_api.dll`)** | `BUILD_VERIFIED` | Compiled with `clang`/`lld`, exported symbols checked, executable test linked and run. | Run on Linux `.so` and macOS `.dylib` in CI matrix. |
| **Godot 4.x GDExtension** | `BUILD_VERIFIED` | Manifest `worldvm.gdextension` written, GDScript `WorldVM.gd` wrapper written, C ABI compatible. | Headless Godot binary execution in CI (`godot --headless --script test.gd`). |
| **Unity Engine 2022.3+ (UPM)** | `UNIT_VERIFIED` | C# P/Invoke bridge `WorldVM.cs`, `WorldVMBehaviour.cs`, struct layouts validated. | Automated Unity test runner batchmode execution (`Unity.exe -batchmode -runTests`). |
| **Unreal Engine 5.3+** | `INTEGRATION_READY_UNVERIFIED` | `.uplugin` descriptor, `WorldVMSubsystem.h`, build targets created. | Build inside `UnrealBuildTool` (UBT) and test in Unreal Editor headless mode. |
| **WASM Sandbox (`Wasmtime 48`)** | `SIMULATION_VERIFIED` | Fuel metering, epoch interruption, memory limit enforcement, instruction traps, SSRF denial. | AOT precompilation benchmarking (`cranelift` compilation caching). |
| **Multiplayer Determinism** | `SIMULATION_VERIFIED` | Cryptographic SHA-256 state hashing across multi-node runs, desync injection detection. | Integration into commercial lockstep / rollback netcodes (e.g. GGRS, Photon Quantum). |

---

## Action Plan to Close Integration Gaps

### 1. Godot 4 Headless Test Runner

- **Action**: Add GitHub Actions CI runner step downloading `godot-headless` for Linux/Windows.
- **Verification**: Run `godot --headless --script sdk/godot/tests/test_worldvm.gd` to promote Godot to `SIMULATION_VERIFIED`.

### 2. Unity Test Runner Batchmode

- **Action**: Configure a Unity Package Manager (UPM) sample project inside `sdk/unity/TestProject`.
- **Verification**: Execute `Unity -batchmode -nographics -projectPath sdk/unity/TestProject -runTests` in a cloud runner with a valid Unity license to promote Unity to `SIMULATION_VERIFIED`.

### 3. Unreal Engine 5 Subsystem Build

- **Action**: Run `GenerateProjectFiles.bat` and compile `WorldVMSubsystem` using Unreal Engine 5.4 UBT.
- **Verification**: Load sample map with a UGC actor and call `InitWorldVM()`, promoting Unreal to `BUILD_VERIFIED` and subsequently `SIMULATION_VERIFIED`.

### 4. Mobile ARM64 Targets (Android / iOS)

- **Action**: Cross-compile `worldvm-runtime` and `worldvm-c-api` for `aarch64-linux-android` and `aarch64-apple-ios`.
- **Verification**: Verify Wasmtime memory bounds and fuel consumption on mobile chipsets with 32-bit linear memory constraints.
