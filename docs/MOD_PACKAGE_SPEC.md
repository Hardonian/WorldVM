# `.worldmod` Package Specification (v1.0)

## Overview

The `.worldmod` format is a standardized, self-contained archive for distributing user-generated gameplay modifications across game engines.

---

## 1. Archive Structure

A valid `.worldmod` package is a standard ZIP archive containing:

```text
my-game-mode.worldmod
├── manifest.toml        # [Mandatory] Package metadata and permission requests
├── module.wasm          # [Mandatory] Compiled WebAssembly binary
├── signature.sig        # [Optional/Mandatory in Prod] Ed25519 cryptographic signature
└── assets/              # [Optional] Mod assets (textures, sounds, localization)
    ├── icon.png
    └── sounds/
        └── alert.wav
```

---

## 2. Mandatory Manifest Schema (`manifest.toml`)

```toml
# Package Identity
name = "my-custom-mode"
version = "1.0.0"
publisher = "creator_username"
worldvm = "1"
abi = "1.0"

# Resource Constraints Requested by Mod
[resources]
memory_mb = 16          # Max linear memory (8MB - 64MB)
fuel = 200000           # Instruction fuel per event
max_execution_ms = 4    # Maximum wall-clock epoch duration

# Explicit Capabilities Requested
[permissions]
request = [
  "world.spawn",
  "ui.notify"
]

# Event Subscriptions
[events]
subscribe = [
  "round_start",
  "tick",
  "player_join"
]
```

---

## 3. Cryptographic Signature Generation

1. **Entry Sorting**: All files in the archive (except `signature.sig`) are sorted in strict lexicographical order.
2. **Canonical Hash**: A SHA-256 digest is calculated over entry names and file byte contents.
3. **Ed25519 Signing**: The SHA-256 digest is signed using the creator's private key.
4. **Signature File**: The 64-byte signature is encoded into `signature.sig`.

---

## 4. Security Constraints on Ingestion

During `.worldmod` unpack and validation, the host engine verifies:

1. **Zip Slip Protection**: Rejects any file containing `..`, absolute paths (e.g. `/etc/passwd`), or drive letters (`C:\`).
2. **Decompression Bomb Guard**: Rejects archives where uncompressed size exceeds 128MB or compression ratio exceeds 50:1.
3. **Native Binary Ban**: Rejects any archive containing dynamic libraries or executables (`.dll`, `.so`, `.dylib`, `.exe`, `.bat`, `.sh`).
4. **WASM Magic Header**: Validates that `module.wasm` begins with `\0asm` (0x00, 0x61, 0x73, 0x6D).
