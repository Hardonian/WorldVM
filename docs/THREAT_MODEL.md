# WorldVM Threat Model & Attack Surface Analysis

## 1. Adversary Profiles

| Adversary Profile | Motivation | Attack Vectors | WorldVM Defense |
|---|---|---|---|
| **Malicious UGC Creator** | Execute arbitrary code on players' PCs; steal Steam tokens, cryptocurrency, or game assets. | Malformed WASM, native binaries inside zip, Zip Slip, network calls to command-and-control servers. | Strict WASM sandbox, WASI denial, native library rejection, path traversal sanitization, SSRF filters. |
| **Griefer / Cheater** | Gain unfair competitive advantage (god mode, speed hacks, item spawning, wallhacks). | Capability escalation, forging entity handles, calling economy APIs from client. | Server-authoritative contract checks, opaque non-sequential ID validation, capability location enforcement. |
| **Denial-of-Service Attacker** | Freeze server instances or player machines; cause lag spikes or server crashes. | Infinite loops, recursion bombs, excessive memory allocations, host call flooding. | Instruction fuel limits, epoch interruption, maximum memory bounds, per-tick call limits, circuit breaker. |
| **Careless Mod Developer** | Unintentional bugs, divide-by-zero, unhandled panics. | Unhandled exceptions or out-of-bounds array access. | Isolated module trap containment; host engine remains unaffected; mod is disabled after 3 consecutive failures. |

---

## 2. Attack Surface Boundaries

### Surface A: `.worldmod` Container Ingestion
- **Risks**: Archive bombs, path traversal (`../../windows/system32`), symlink attacks, embedded `.exe` or `.dll`.
- **Mitigation**: Pure-Rust ZIP reader with maximum entry size limit, compression ratio check, strict filename sanitation (no `..` or leading `/`), and immediate rejection of any binary executables.

### Surface B: Guest WASM Compilation & Instantiation
- **Risks**: JIT vulnerabilities, compiler hangs on deeply nested expressions, massive memory pre-allocation.
- **Mitigation**: Wasmtime Cranelift compiler with deterministic limits, memory pre-allocation caps, and rejection of unsupported WASM proposals (e.g. threads).

### Surface C: Guest-to-Host ABI Boundary
- **Risks**: Buffer overflow, invalid UTF-8 strings, out-of-bounds pointer passing from guest memory, re-entrancy attacks.
- **Mitigation**:
  - Memory bounds verification: host always bounds-checks guest pointers against linear memory size.
  - No raw pointers: all data is serialized as JSON or fixed-layout structs across the boundary.
  - Re-entrancy guard: events are dispatched iteratively, not recursively.

### Surface D: Capability Invocation
- **Risks**: Privilege escalation, calling ungranted methods, parameter pollution.
- **Mitigation**: Multi-tier capability enforcement checking module permissions, game contract exposure, execution location, and call quotas per simulation tick.

---

## 3. Residual Risks & Studio Responsibilities

1. **Game Logic Flaws**: If a studio defines a capability `player.teleport` without server-side validation of coordinates, a creator with that capability can teleport players outside map boundaries. Studios must validate business logic inside their `WorldCapabilityProvider`.
2. **Asset VRAM Consumption**: While WorldVM enforces byte size limits on `.worldmod` archives, studios must inspect high-resolution texture or polygon budgets during asset loading in their rendering pipeline.
