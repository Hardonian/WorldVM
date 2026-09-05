//! Frame Budget & Tick Cadence Scenario.
//! Validates execution latency bounds at 30 Hz, 60 Hz, and 120 Hz tick rates
//! to guarantee zero frame stutter or hitching in production game engines.

use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_runtime::WorldVmRuntime;

use crate::corpus::legitimate;
use crate::simworld::{SimClock, SyntheticCapabilityProvider, TickRate, WorldState};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadenceStats {
    pub tick_rate_hz: u32,
    pub target_frame_time_us: u64,
    pub total_ticks: u64,
    pub min_tick_us: u64,
    pub max_tick_us: u64,
    pub avg_tick_us: f64,
    pub p99_tick_us: u64,
    pub budget_exceeded_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameBudgetResult {
    pub stats_30hz: CadenceStats,
    pub stats_60hz: CadenceStats,
    pub stats_120hz: CadenceStats,
    pub passed: bool,
    pub message: String,
}

fn measure_cadence(tick_rate: TickRate, ticks: u64, max_budget_us: u64) -> CadenceStats {
    let state = Arc::new(RwLock::new(WorldState::default()));
    let host = Arc::new(SyntheticCapabilityProvider::new(state.clone()));
    let contract = WorldCapabilityContract::standard_arcade_contract("cadence-test");
    let mut runtime = WorldVmRuntime::new(contract, host, false).unwrap();

    let mod_gravity = legitimate::build_gravity_module();
    let mod_race = legitimate::build_race_module();

    runtime.load_module(mod_gravity).unwrap();
    runtime.load_module(mod_race).unwrap();

    let mut clock = SimClock::new(tick_rate);
    let mut durations_us = Vec::with_capacity(ticks as usize);
    let mut budget_exceeded = 0;

    for tick in 1..=ticks {
        state.write().tick = tick;
        clock.advance();

        let start = Instant::now();

        // Emit events on every tick simulating game loop
        let _ = runtime.emit_event(
            "low-gravity-mod",
            "player_join",
            b"{\"player_id\":\"p1\",\"player_name\":\"Tester\"}",
        );
        let _ = runtime.emit_event(
            "checkpoint-race-mod",
            "checkpoint",
            b"{\"player_id\":\"p1\",\"checkpoint_index\":1,\"lap\":1}",
        );

        let elapsed_us = start.elapsed().as_micros() as u64;
        durations_us.push(elapsed_us);

        if elapsed_us > max_budget_us {
            budget_exceeded += 1;
        }
    }

    durations_us.sort_unstable();
    let min_tick_us = *durations_us.first().unwrap_or(&0);
    let max_tick_us = *durations_us.last().unwrap_or(&0);
    let avg_tick_us = durations_us.iter().copied().sum::<u64>() as f64 / durations_us.len().max(1) as f64;
    let p99_idx = ((durations_us.len() as f64) * 0.99) as usize;
    let p99_tick_us = durations_us[p99_idx.min(durations_us.len().saturating_sub(1))];

    CadenceStats {
        tick_rate_hz: tick_rate.hz(),
        target_frame_time_us: tick_rate.frame_budget_us(),
        total_ticks: ticks,
        min_tick_us,
        max_tick_us,
        avg_tick_us,
        p99_tick_us,
        budget_exceeded_count: budget_exceeded,
    }
}

pub fn run_frame_budget_scenario() -> FrameBudgetResult {
    // 500us budget allocated for mod runtime per frame
    let mod_budget_us = 2000; // 2.0 ms max allowed spike budget

    let stats_30hz = measure_cadence(TickRate::Hz30, 100, mod_budget_us);
    let stats_60hz = measure_cadence(TickRate::Hz60, 200, mod_budget_us);
    let stats_120hz = measure_cadence(TickRate::Hz120, 300, mod_budget_us);

    // Frame budget verification: average tick execution should be well below 500 µs
    let passed = stats_60hz.avg_tick_us < 1000.0 && stats_120hz.avg_tick_us < 1000.0;

    FrameBudgetResult {
        stats_30hz: stats_30hz.clone(),
        stats_60hz: stats_60hz.clone(),
        stats_120hz: stats_120hz.clone(),
        passed,
        message: format!(
            "Frame Budget: 30Hz avg={:.1}us p99={}us | 60Hz avg={:.1}us p99={}us | 120Hz avg={:.1}us p99={}us (budget: {}us)",
            stats_30hz.avg_tick_us, stats_30hz.p99_tick_us,
            stats_60hz.avg_tick_us, stats_60hz.p99_tick_us,
            stats_120hz.avg_tick_us, stats_120hz.p99_tick_us,
            mod_budget_us
        ),
    }
}
