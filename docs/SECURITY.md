# WorldVM Security Architecture & Guarantees

## Security Principles

WorldVM operates under a **zero-trust execution model**. Creator-submitted gameplay code is assumed to be potentially buggy, adversarial, or actively hostile. The sandbox guarantees that no untrusted module can compromise the host process, crash the game engine, steal player credentials, or exploit local network topology.

---

## 1. WebAssembly Memory Isolation

1. **Strict Linear Memory**: Modules execute within an isolated linear memory buffer (default 16MB, max configurable up to 64MB).
2. **No Pointer Dereferencing**: WASM memory addresses are offsets into an array; out-of-bounds reads or writes trigger a deterministic WASM trap rather than a host segfault.
3. **No Direct Host Pointers**: Host pointers are never passed into the sandbox. All interactions occur via opaque 64-bit integer handles (`PlayerId(u64)`, `EntityId(u64)`).

---

## 2. Resource Exhaustion Protections

| Attack Vector | Defense Mechanism | Behavior on Violation |
| --- | --- | --- |
| **Infinite Loops** (`while(true) {}`) | Instruction Fuel Metering | Traps immediately when fuel reaches 0 (`OutOfFuel`). Execution aborts within <1ms. |
| **Blocking Sleep / Spinlocks** | Epoch-based Deadlines | Engine background thread increments epoch counter. If deadline expires, guest traps. |
| **Linear Memory Flooding** | Maximum Page Limits | Module trap occurs when attempting `memory.grow` beyond configured `memory_mb`. |
| **Host Call Storms** | Per-Tick Rate Limits | CapabilityEnforcer returns `RateLimitExceeded`. Host call is not executed. |
| **Decompression Bombs** | ZIP extraction guards | Packages exceeding 128MB uncompressed size or a 50:1 compression ratio are rejected. |
| **Path Traversal** (`../../shadow`) | Canonical Zip Slip check | Rejected on inspection (`Malicious path traversal detected`). |

---

## 3. Capability Enforcement Matrix

Every capability call made by a guest module undergoes a multi-phase check before reaching game engine code:

1. **Module Manifest Request**: The mod's `manifest.toml` must explicitly declare `[permissions.request]`.
2. **Host Game Policy Exposure**: The game studio's `game_policy.yaml` contract must expose the capability.
3. **Execution Location Check**:
   - `CapabilityLocation::Server`: Denied when executed on client runtimes (`is_server = false`).
   - `CapabilityLocation::Client`: Denied when executed on headless dedicated servers.
   - `CapabilityLocation::Both`: Permitted on either.
4. **Rate Limit Enforcement**: Calls per tick must not exceed `calls_per_tick`.
5. **Argument Sanitization**: For sensitive operations like `world.spawn`, entity types must belong to an explicit allowlist.

---

## 4. Network SSRF Prevention

WorldVM prevents Server-Side Request Forgery (SSRF) and local network enumeration:

- **Private Subnet Denial**: Any request to `127.0.0.1`, `localhost`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, or `169.254.169.254` (cloud metadata) is rejected at the capability layer.
- **Domain Allowlists**: Studios can restrict network access to an explicit list of trusted endpoints (e.g. `api.mygame.com`).

---

## 5. Cryptographic Packaging & Trust Chains

Mods are distributed in signed `.worldmod` containers:

- **Canonical SHA-256 Digest**: Computed over all archive entries in alphabetical order (excluding the signature file itself).
- **Ed25519 Signatures**: Verified against studio public keys or verified creator identities.
- **Tamper Evident**: Modifying even a single byte of WASM code or manifest configuration invalidates the signature.
