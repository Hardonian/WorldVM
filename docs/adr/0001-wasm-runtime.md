# ADR 0001: WebAssembly Runtime Engine Selection

## Status
Accepted

## Context
WorldVM requires a sandboxed WebAssembly (WASM) execution engine capable of running untrusted, creator-authored gameplay code across desktop game clients (Godot, Unity, Unreal, custom C/C++ engines) and headless game servers. 

Key technical requirements for the runtime include:
1. **Deterministic Resource Metering**: Instruction-level fuel metering to strictly bound CPU execution time without relying on asynchronous OS thread preemption.
2. **Execution Deadlines & Interruptions**: Epoch-based interruption or deadlines to ensure creator scripts cannot stall the host game loop (e.g. strict 2-5ms frame limits).
3. **Linear Memory Isolation**: Enforceable maximum memory allocations per module instance (e.g. 16MB - 64MB) with guaranteed out-of-memory traps that do not crash or corrupt the host engine.
4. **WASI Isolation & Virtualization**: Complete capability to deny host filesystem, raw sockets, process spawning, and arbitrary environment variables, while providing a strictly controlled capability-oriented host ABI.
5. **JIT & AOT Performance**: High-efficiency Cranelift compilation with module caching so that game initialization and mod loading times remain imperceptible.
6. **Embeddability**: Clean Rust API, C ABI exportability, minimal runtime dependencies, and cross-platform support across Windows, Linux, and macOS.
7. **Standards Compliance & Component Model Trajectory**: Active alignment with Bytecode Alliance standards and the WASM Component Model.

## Candidates Evaluated

### 1. Wasmtime (Bytecode Alliance)
- **Strengths**: 
  - Standard-bearer for WASM security maintained by the Bytecode Alliance.
  - Native instruction fuel metering (`Store::set_fuel`, `Config::consume_fuel(true)`) allowing exact deterministic CPU budgeting.
  - Epoch-based interruption (`Config::epoch_interruption(true)`) allowing external timers to interrupt runaway code safely across threads.
  - Granular resource limiting (`ResourceLimiter`) for instance memory, table elements, and instances.
  - Modular WASI crate (`wasmtime-wasi`) allowing complete removal or sandboxing of system capabilities.
  - Production-proven across Fastly, Shopify, and enterprise edge workers.
- **Weaknesses**:
  - Requires Cranelift JIT compilation on desktop/server targets; requires pre-compilation (AOT) for platforms prohibiting JIT (e.g., iOS/consoles).

### 2. Wasmer
- **Strengths**: Multi-backend support (Cranelift, LLVM, Singlepass).
- **Weaknesses**: Commercial focus shifts, less predictable release cadence for core runtime embeddings, more complex API migration between major versions, less uniform fuel/epoch interruption integration compared to Wasmtime.

### 3. Wazero
- **Strengths**: Pure Go zero-dependency runtime.
- **Weaknesses**: Writing the core engine in Go would introduce a Go runtime / GC overhead into C/C++/Rust game engines, which is unacceptable for frame-critical game client embedding.

### 4. Custom WASM Interpreter
- **Weaknesses**: Violates core rule ("Do not implement a custom VM"). Fragile, slow, and lacks formal security audits.

## Decision
We select **Wasmtime** as the primary WebAssembly execution engine for WorldVM.

Wasmtime delivers the highest security assurances, deterministic instruction fuel metering, epoch-based execution deadlines, fine-grained memory limiting, and robust embedding capabilities in Rust and via C ABI for Godot, Unity, Unreal, and custom game engines.

## Consequences
- Core runtime crates (`worldvm-runtime`) will link to `wasmtime`.
- Deterministic execution profiles will configure fuel limits and deterministic clocks via Wasmtime's host environment.
- Any game client embedding WorldVM will utilize Cranelift JIT or pre-compiled `.cwasm` artifacts.
- Untrusted WASI calls are completely denied; host capabilities are exclusively routed through the WorldVM ABI v1.
