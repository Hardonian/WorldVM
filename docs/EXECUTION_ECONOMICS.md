# WorldVM Execution Economics & Metering

## Overview

In user-generated content (UGC) gaming platforms, resource metering is essential for two reasons:
1. **Engine Frame Budget**: Game engines targeting 60 Hz or 120 Hz have fixed frame budgets (16.6ms and 8.3ms respectively). Mod execution must fit within a fraction of this budget (typically 500µs to 2ms).
2. **Server Infrastructure Costs**: Headless dedicated servers running hundreds of community mods must allocate CPU and memory deterministically to prevent noisy neighbors and runaway cloud costs.

---

## 1. Instruction Fuel Metering

WorldVM tracks execution progress via **WASM instruction fuel**. Every bytecode operation decrements the store's remaining fuel according to instruction weight:
- Simple math and register moves: 1 unit
- Memory load / store: 2–3 units
- Function calls: 5 units
- Memory growth: 50 units

### Standard Fuel Allocations

| Workload Tier | Fuel Budget per Event | Equivalent Host Time | Intended Use Case |
|---|---|---|---|
| **Micro** | 50,000 units | ~0.2 – 0.4 ms | Lightweight event triggers (UI notifications, sound triggers). |
| **Standard** | 200,000 units | ~0.5 – 1.0 ms | Standard game modes (racing checkpoints, gravity modifiers). |
| **Heavy** | 500,000 units | ~1.5 – 3.0 ms | Complex AI wave spawning, path calculation, quest logic. |

When remaining fuel reaches zero:
- The WASM virtual machine immediately halts.
- A `WorldVmError::OutOfFuel` error is returned to the host.
- The host frame rate is protected with zero frame drops.

---

## 2. Epoch Interruption Deadlines

For host environments where wall-clock guarantees are required in addition to instruction counts (e.g. host calls that take measurable time), WorldVM uses **epoch deadlines**:
- An engine background timer increments the global epoch counter at a fixed interval (e.g. every 1ms).
- When a guest event begins, its epoch deadline is set to `current_epoch + max_duration`.
- If the deadline expires while inside guest code, the runner triggers an immediate interruption trap.

---

## 3. Cryptographic Execution Receipts

On dedicated servers, WorldVM generates verifiable execution receipts (`ExecutionReceipt`):
```json
{
  "receipt_id": "rcpt_9f81a7b2",
  "match_id": "match_55102",
  "module_id": "zombie-survival-mod",
  "tick": 1840,
  "metrics": {
    "invocations": 1,
    "fuel_consumed": 48200,
    "execution_time_us": 142,
    "host_calls": 3,
    "errors_encountered": 0
  },
  "content_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "host_signature": "ed25519_sig_..."
}
```
Studios can use these receipts for server billing, creator payout calculations, and cheat dispute auditing.
