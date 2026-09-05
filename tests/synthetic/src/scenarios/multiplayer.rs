//! Multiplayer Consistency, Determinism & Desync Scenarios.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_runtime::WorldVmRuntime;

use crate::corpus::legitimate;
use crate::simworld::{SimClock, SimPlayer, SimRng, SyntheticCapabilityProvider, TickRate, WorldState};

pub fn compute_state_hash(state: &WorldState) -> String {
    let mut hasher = Sha256::new();
    hasher.update(state.match_id.as_bytes());
    hasher.update(&state.tick.to_le_bytes());
    hasher.update(&state.gravity.to_le_bytes());

    let mut player_ids: Vec<_> = state.players.keys().cloned().collect();
    player_ids.sort();
    for pid in player_ids {
        let p = &state.players[&pid];
        hasher.update(p.id.as_bytes());
        hasher.update(&p.x.to_le_bytes());
        hasher.update(&p.y.to_le_bytes());
        hasher.update(&p.z.to_le_bytes());
        hasher.update(&p.health.to_le_bytes());
        hasher.update(&p.xp.to_le_bytes());
    }

    let mut entity_ids: Vec<_> = state.entities.keys().cloned().collect();
    entity_ids.sort();
    for eid in entity_ids {
        let e = &state.entities[&eid];
        hasher.update(&e.id.to_le_bytes());
        hasher.update(e.entity_type.as_bytes());
        hasher.update(&e.x.to_le_bytes());
        hasher.update(&e.z.to_le_bytes());
    }

    hex::encode(hasher.finalize())
}

/// Runs a deterministic multiplayer match simulation.
fn run_match_instance(seed: u64, mutate_event: bool) -> (String, u64) {
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state.clone()));
    let contract = WorldCapabilityContract::standard_arcade_contract("sim-game");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    let mod_gravity = legitimate::build_gravity_module();
    let mod_survival = legitimate::build_survival_module();

    runtime.load_module(mod_gravity).unwrap();
    runtime.load_module(mod_survival).unwrap();

    let mut rng = SimRng::from_seed(seed);
    let mut clock = SimClock::new(TickRate::Hz60);

    // Initial 8 players
    for i in 1..=8 {
        let pid = format!("player_{i}");
        state.write().players.insert(
            pid.clone(),
            SimPlayer {
                id: pid.clone(),
                name: format!("Racer_{i}"),
                team_id: (i % 2) as u32,
                x: rng.range_f32(-10.0, 10.0),
                y: 0.0,
                z: rng.range_f32(-10.0, 10.0),
                health: 100.0,
                max_health: 100.0,
                xp: 0,
                score: 0,
                inventory: HashMap::new(),
                notifications: Vec::new(),
            },
        );
    }

    // 100 simulated game ticks
    let mut executions = 0;
    for tick in 1..=100 {
        state.write().tick = tick;
        clock.advance();

        if tick == 1 {
            let _ = runtime.emit_event("low-gravity-mod", "round_start", b"{}");
            let _ = runtime.emit_event("zombie-survival-mod", "round_start", b"{}");
            executions += 2;
        }

        // Mid-game event
        if tick == 50 {
            if mutate_event {
                // Desync injection: divergent state mutation (e.g., unverified client prediction)
                if let Some(p) = state.write().players.get_mut("player_1") {
                    p.x += 2.5;
                }
            }
            let _ = runtime.emit_event(
                "low-gravity-mod",
                "player_join",
                b"{\"player_id\":\"player_1\",\"player_name\":\"Racer_1\"}",
            );
            executions += 1;
        }

        // Ticks for survival mod
        if tick % 10 == 0 {
            let _ = runtime.emit_event("zombie-survival-mod", "tick", b"{}");
            executions += 1;
        }
    }

    let final_hash = compute_state_hash(&state.read());
    (final_hash, executions)
}

pub fn test_multiplayer_determinism(seed: u64) -> (bool, String, String) {
    let (hash_a, _) = run_match_instance(seed, false);
    let (hash_b, _) = run_match_instance(seed, false);

    (hash_a == hash_b, hash_a, hash_b)
}

pub fn test_desync_detection(seed: u64) -> (bool, String, String) {
    let (hash_clean, _) = run_match_instance(seed, false);
    let (hash_mutated, _) = run_match_instance(seed, true);

    (hash_clean != hash_mutated, hash_clean, hash_mutated)
}
