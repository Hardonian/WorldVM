# WorldVM Developer Quickstart

## 1. Prerequisites

- **Rust**: 1.80+ (`rustup toolchain install stable`)
- **WebAssembly Target**: `rustup target add wasm32-unknown-unknown`
- **C Compiler (Optional)**: `clang` or `gcc` for compiling C ABI samples

---

## 2. Install WorldVM CLI

Build and install the `worldvm` CLI binary:

```bash
cargo install --path crates/worldvm-cli
```

Verify your installation:

```bash
worldvm doctor
```

---

## 3. Create a New Mod

Initialize a new creator mod project:

```bash
worldvm init my-gravity-mod
cd my-gravity-mod
```

This generates a project structure:

```text
my-gravity-mod/
├── Cargo.toml
├── manifest.toml
└── src/
    └── lib.rs
```

---

## 4. Write Gameplay Logic (Rust SDK)

Open `src/lib.rs`:

```rust
use worldvm_sdk::prelude::*;

#[derive(Default)]
pub struct LowGravityMod;

impl WorldVmEntrypoint for LowGravityMod {
    fn on_event(&mut self, event_name: &str, _payload: &[u8]) -> Result<(), String> {
        match event_name {
            "round_start" => {
                // Set low moon gravity (2.40 m/s^2)
                world::set_gravity(2.40).map_err(|e| e.to_string())?;
                // Send toast notification to all players
                ui::notify_player("all", "Lunar Gravity Activated! (2.4 m/s^2)").map_err(|e| e.to_string())?;
            }
            _ => {}
        }
        Ok(())
    }
}

export_entrypoint!(LowGravityMod);
```

---

## 5. Build, Package & Verify

Compile to WebAssembly:

```bash
worldvm build
```

Package into a `.worldmod` archive:

```bash
worldvm package --out dist/my-gravity-mod.worldmod
```

Inspect the package:

```bash
worldvm inspect dist/my-gravity-mod.worldmod
```

---

## 6. Run in Standalone Simulator

Test your mod locally against an in-memory game host without launching the full game engine:

```bash
worldvm test dist/my-gravity-mod.worldmod --event round_start
```

---

## 7. Run Reference Game ("Neon Arena")

See multiple mods interacting in a real 60 Hz physics loop:

```bash
cargo run -p reference-game
```
