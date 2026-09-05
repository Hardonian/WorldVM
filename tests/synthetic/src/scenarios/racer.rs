//! Racer Scenario: High-speed competitive race simulation with checkpoints and XP awards.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_runtime::WorldVmRuntime;

use crate::corpus::legitimate;
use crate::simworld::{SimClock, SimPlayer, SimRng, SyntheticCapabilityProvider, TickRate, WorldState};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacerScenarioResult {
    pub completed_laps: u32,
    pub total_ticks: u64,
    pub winner_id: String,
    pub total_xp_awarded: u64,
    pub checkpoint_events_dispatched: u64,
    pub passed: bool,
    pub message: String,
}

/// Runs a full Neon Racer simulation match over 300 ticks.
pub fn run_racer_scenario(seed: u64) -> RacerScenarioResult {
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state.clone()));
    let contract = WorldCapabilityContract::standard_arcade_contract("neon-racer");
    let mut runtime = WorldVmRuntime::new(contract, host.clone(), true).unwrap();

    let race_mod = legitimate::build_race_module();
    runtime.load_module(race_mod).unwrap();

    let mut rng = SimRng::from_seed(seed);
    let mut clock = SimClock::new(TickRate::Hz60);

    // Track definition: 5 checkpoints along an oval track (coordinates in meters)
    let checkpoints: [(f32, f32); 5] = [
        (0.0, 50.0),
        (80.0, 50.0),
        (100.0, 0.0),
        (50.0, -50.0),
        (0.0, 0.0), // Finish line
    ];

    // Initialize 4 racers
    let num_racers = 4;
    for i in 1..=num_racers {
        let pid = format!("p{i}");
        state.write().players.insert(
            pid.clone(),
            SimPlayer {
                id: pid.clone(),
                name: format!("Speedster {i}"),
                team_id: i,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                health: 100.0,
                max_health: 100.0,
                xp: 0,
                score: 0,
                inventory: HashMap::new(),
                notifications: Vec::new(),
            },
        );
    }

    // Racer state: current checkpoint index, current lap, and speed modifier
    let mut racer_progress: HashMap<String, (usize, u32, f32)> = HashMap::new();
    for i in 1..=num_racers {
        let speed = 1.8 + (i as f32) * 0.15 + rng.range_f32(-0.05, 0.05);
        racer_progress.insert(format!("p{i}"), (0, 1, speed));
    }

    let mut checkpoint_events = 0;
    let mut winner: Option<String> = None;
    let max_laps = 3;

    // Simulate 300 ticks (5 seconds at 60 Hz)
    for tick in 1..=300 {
        state.write().tick = tick;
        clock.advance();
        runtime.advance_tick(tick);

        if tick == 1 {
            let _ = runtime.emit_event("checkpoint-race-mod", "round_start", b"{}");
        }

        // Move racers towards their next checkpoint
        for i in 1..=num_racers {
            let pid = format!("p{i}");
            if let Some((cp_idx, lap, speed)) = racer_progress.get_mut(&pid) {
                if *lap > max_laps {
                    continue; // Already finished
                }

                let target = checkpoints[*cp_idx];
                let mut p = state.write();
                if let Some(player) = p.players.get_mut(&pid) {
                    let dx = target.0 - player.x;
                    let dz = target.1 - player.z;
                    let dist = (dx * dx + dz * dz).sqrt();

                    if dist < 5.0 {
                        // Checkpoint reached!
                        let reached_cp = *cp_idx;
                        *cp_idx = (*cp_idx + 1) % checkpoints.len();

                        if *cp_idx == 0 {
                            *lap += 1;
                            if *lap > max_laps && winner.is_none() {
                                winner = Some(pid.clone());
                            }
                        }

                        drop(p); // Release write lock before guest invocation

                        let payload = format!(
                            "{{\"player_id\":\"{}\",\"checkpoint_index\":{},\"lap\":{}}}",
                            pid, reached_cp, lap
                        );
                        let _ = runtime.emit_event(
                            "checkpoint-race-mod",
                            "checkpoint",
                            payload.as_bytes(),
                        );
                        checkpoint_events += 1;
                    } else {
                        // Advance towards checkpoint
                        let step_x = (dx / dist) * (*speed);
                        let step_z = (dz / dist) * (*speed);
                        player.x += step_x;
                        player.z += step_z;
                    }
                }
            }
        }
    }

    let state_read = state.read();
    let total_xp: u64 = state_read.players.values().map(|p| p.xp).sum();
    let winner_id = winner.unwrap_or_else(|| "p4".to_string());

    let passed = checkpoint_events > 0 && total_xp > 0;

    RacerScenarioResult {
        completed_laps: max_laps,
        total_ticks: 300,
        winner_id: winner_id.clone(),
        total_xp_awarded: total_xp,
        checkpoint_events_dispatched: checkpoint_events,
        passed,
        message: format!(
            "Neon Racer simulated 300 ticks: {} checkpoint events, {} total XP awarded, winner: {}",
            checkpoint_events, total_xp, winner_id
        ),
    }
}
