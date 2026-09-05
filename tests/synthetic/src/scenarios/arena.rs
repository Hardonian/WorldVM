//! Arena Deathmatch & Zombie Survival Scenario.
//! Simulates wave-based zombie combat, player damage, entity lifecycle, and survival score.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_runtime::WorldVmRuntime;

use crate::corpus::legitimate;
use crate::simworld::{SimClock, SimPlayer, SimRng, SyntheticCapabilityProvider, TickRate, WorldState};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaScenarioResult {
    pub total_ticks: u64,
    pub zombies_spawned: usize,
    pub player_damage_events: u64,
    pub players_survived: usize,
    pub total_score: i64,
    pub passed: bool,
    pub message: String,
}

/// Runs a full Arena wave combat simulation over 500 ticks.
pub fn run_arena_scenario(seed: u64) -> ArenaScenarioResult {
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state.clone()));
    let contract = WorldCapabilityContract::standard_arcade_contract("neon-arena");
    let mut runtime = WorldVmRuntime::new(contract, host.clone(), false).unwrap();

    let survival_mod = legitimate::build_survival_module();
    runtime.load_module(survival_mod).unwrap();

    let mut rng = SimRng::from_seed(seed);
    let mut clock = SimClock::new(TickRate::Hz60);

    // Initialize 4 arena gladiators
    for i in 1..=4 {
        let pid = format!("player_{i}");
        state.write().players.insert(
            pid.clone(),
            SimPlayer {
                id: pid.clone(),
                name: format!("Gladiator {i}"),
                team_id: 1,
                x: rng.range_f32(-15.0, 15.0),
                y: 0.0,
                z: rng.range_f32(-15.0, 15.0),
                health: 100.0,
                max_health: 100.0,
                xp: 0,
                score: 0,
                inventory: HashMap::new(),
                notifications: Vec::new(),
            },
        );
    }

    let mut player_damage_events = 0;

    // Simulate 500 ticks
    for tick in 1..=500 {
        state.write().tick = tick;
        clock.advance();

        // Round start at tick 1 triggers initial wave spawn
        if tick == 1 {
            let _ = runtime.emit_event("zombie-survival-mod", "round_start", b"{}");
        }

        // Every 30 ticks (0.5s at 60Hz), the survival mod runs its wave logic
        if tick % 30 == 0 {
            let _ = runtime.emit_event("zombie-survival-mod", "tick", b"{}");
        }

        // Every 60 ticks, simulate combat interactions: zombies attack nearby players
        if tick % 60 == 0 {
            let mut state_lock = state.write();
            let mut damaged_players = Vec::new();

            for (pid, player) in state_lock.players.iter_mut() {
                if player.health > 0.0 {
                    // Zombie claw attack
                    let damage = rng.range_f32(5.0, 15.0);
                    player.health = (player.health - damage).max(0.0);
                    player.score += 50; // Points for surviving wave
                    damaged_players.push((pid.clone(), damage));
                }
            }
            drop(state_lock);

            for (pid, dmg) in damaged_players {
                player_damage_events += 1;
                let payload = format!("{{\"player_id\":\"{}\",\"damage\":{:.1}}}", pid, dmg);
                let _ = runtime.emit_event("zombie-survival-mod", "player_damage", payload.as_bytes());
            }
        }
    }

    let final_state = state.read();
    let zombies_spawned = final_state.entities.len();
    let survivors = final_state.players.values().filter(|p| p.health > 0.0).count();
    let total_score: i64 = final_state.players.values().map(|p| p.score).sum();

    let passed = zombies_spawned > 0 && player_damage_events > 0;

    ArenaScenarioResult {
        total_ticks: 500,
        zombies_spawned,
        player_damage_events,
        players_survived: survivors,
        total_score,
        passed,
        message: format!(
            "Arena Zombie Survival: {} zombies spawned, {} damage events, {}/4 gladiators survived, total score {}",
            zombies_spawned, player_damage_events, survivors, total_score
        ),
    }
}
