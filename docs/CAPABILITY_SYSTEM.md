# WorldVM Capability Permissions & Policy System

## Overview

WorldVM enforces a capability-based security model. In traditional modding systems (e.g. Lua scripts with global tables), any script can access global game state, overwrite global functions, or trigger crashes. WorldVM replaces this with **fine-grained capability contracts**.

---

## 1. Permission Categories

Capabilities belong to strict functional categories:

| Category | Description | Examples |
| --- | --- | --- |
| **`Read`** | Non-mutating observation of world or player state. | `world.get_gravity`, `player.read_position` |
| **`Write`** | Safe mutations of gameplay state within defined limits. | `world.set_gravity`, `world.spawn` |
| **`Economy`** | Modifications to XP, currency, inventory, or rewards (server-only). | `player.grant_xp`, `inventory.grant` |
| **`Communication`** | Sending messages, HUD notifications, or chat. | `ui.notify`, `chat.broadcast` |
| **`Network`** | External HTTP/WebSocket egress (strictly governed). | `network.http` |
| **`Admin`** | Sensitive operations restricted to official studio staff. | `server.restart`, `match.terminate` |

---

## 2. Studio Game Policy Specification (`game_policy.yaml`)

Studios define their game's capability contract in a clean declarative YAML format:

```yaml
game_id: "neon-arena"
version: "1.0.0"

capabilities:
  world.set_gravity:
    access: Write
    category: Write
    location: Both
    rate_limit:
      calls_per_tick: 4

  world.spawn:
    access: Write
    category: Write
    location: Both
    rate_limit:
      calls_per_tick: 10
      allowed_types:
        - "zombie"
        - "checkpoint"
        - "powerup"

  player.grant_xp:
    access: Write
    category: Economy
    location: Server # Disallowed from client runtimes
    rate_limit:
      calls_per_tick: 2

  ui.notify:
    access: Write
    category: Communication
    location: Both
    rate_limit:
      calls_per_tick: 8
```

---

## 3. Mod Manifest Declaration (`manifest.toml`)

Creators specify the capabilities their mod requires inside `manifest.toml`:

```toml
name = "low-gravity-mod"
version = "1.0.0"
publisher = "neon_creator"
worldvm = "1"
abi = "1.0"

[resources]
memory_mb = 16
fuel = 200000
max_execution_ms = 3

[permissions]
request = [
  "world.set_gravity",
  "ui.notify"
]

[events]
subscribe = [
  "round_start",
  "player_join"
]
```

If a mod attempts to call a capability not listed in `[permissions.request]`, or one not exposed by the host game's policy, the invocation is blocked immediately by the host runtime with zero performance penalty.
