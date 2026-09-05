# WorldVM Architectural Specification

## Executive Summary

**WorldVM** is an embeddable, sandboxed WebAssembly execution environment specifically engineered for real-time video games. It allows game developers to run untrusted, creator-authored gameplay code across client and server environments with deterministic guarantees, strict resource budgets, and zero danger of game engine crashes or host compromise.

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

## Core Tenets

1. **Safety First**: Untrusted creator code can never access arbitrary disk paths, raw sockets, threading primitives, or native system libraries. WASI is explicitly denied.
2. **Deterministic Budgets**: Every event execution is bounded by instruction fuel (e.g. 100,000 instructions) and wall-clock epoch deadlines (e.g. 5 milliseconds). Infinite loops trap cleanly and cannot freeze the frame rate.
3. **Capability-Gated Authority**: Guest modules possess zero ambient authority. All host mutations (spawning entities, awarding XP, setting physics rules, playing audio) require explicitly declared capability permissions that the host game contract must expose.
4. **Multi-Engine Portability**: The same `.worldmod` package runs identically inside Godot, Unity, Unreal, or custom dedicated servers via the stable C ABI.
5. **Zero-Overhead Host Interop**: Low-latency ABI encoding over contiguous linear memory buffers, avoiding multi-megabyte serialization overhead during frame ticks.

---

## Component Topology

### 1. `worldvm-core`

Fundamental data structures, opaque handles (`PlayerId`, `EntityId`, `ModId`), deterministic hashing helpers, error taxonomy (`WorldVmError`), and execution metrics records (`ExecutionMetrics`).

### 2. `worldvm-abi`

Binary layout specifications between host and guest.

- Stable host call function signature:

  ```c
  int32_t worldvm_host_call(
      int32_t cap_name_ptr,
      int32_t cap_name_len,
      int32_t input_payload_ptr,
      int32_t input_payload_len,
      int32_t output_buf_ptr_ptr,
      int32_t output_buf_len_ptr
  );
  ```

- Guest mandatory exports:
  - `worldvm_guest_alloc(size: i32) -> i32`
  - `worldvm_guest_free(ptr: i32, size: i32)`
  - `worldvm_get_abi_version() -> i32`
  - `worldvm_handle_event(name_ptr: i32, name_len: i32, payload_ptr: i32, payload_len: i32) -> i32`

### 3. `worldvm-capabilities`

Permission enforcement layer. Defines `WorldCapabilityContract`, `CapabilityAccess`, `PermissionCategory`, `RateLimitRule`, and `CapabilityEnforcer`.

### 4. `worldvm-signing`

Ed25519 signature generation and verification. Guarantees deterministic content hashing (excluding mutable signature files) to verify package integrity and establish trust chains.

### 5. `worldvm-package`

The `.worldmod` distribution format. Standard ZIP-based archive containing:

- `manifest.toml`: Metadata, target ABI, declared event subscriptions, and required capabilities.
- `module.wasm`: Compiled WebAssembly bytecode.
- `signature.sig`: Ed25519 signature of the module bundle.
- `assets/`: Optional creator textures, audio, models, or data tables.

Includes built-in security protections against Zip Slip path traversal, decompression bombs (>128MB uncompressed), and native dynamic libraries (`.dll`, `.so`, `.dylib`).

### 6. `worldvm-runtime`

The engine that hosts and executes modules using Wasmtime 48.

- Fuel consumption: `consume_fuel(true)` checks instruction limits at branch points.
- Epoch interruption: Background thread ticks epochs to interrupt runaway operations.
- Circuit breaker: Automatically disables offending modules that trigger 3 consecutive unhandled traps.

### 7. `worldvm-c-api`

C-compatible dynamic library (`worldvm_c_api.dll` / `libworldvm_c_api.so`) with header `worldvm.h` enabling seamless P/Invoke into Unity C#, GDExtension into Godot, and C++ into Unreal Engine 5.

### 8. `worldvm-cli`

Command-line developer tool for creators and studios: `init`, `build`, `package`, `inspect`, `verify`, `sign`, `test`, `dev`, `doctor`.
