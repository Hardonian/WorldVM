//! Synthetic Lab Report Generator.
//! Produces machine-readable JSON artifacts and executive Markdown summaries
//! from actual test execution data with zero fabricated results.

use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::chaos::injector::ChaosResult;
use crate::scenarios::arena::ArenaScenarioResult;
use crate::scenarios::frame_budget::FrameBudgetResult;
use crate::scenarios::racer::RacerScenarioResult;
use crate::scenarios::red_team::RedTeamResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineEvidenceEntry {
    pub engine: String,
    pub classification: String,
    pub test_target: String,
    pub status: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub execution_time_ms: u64,
    pub pass_rate_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticReport {
    pub title: String,
    pub version: String,
    pub execution_timestamp: String,
    pub profile: String,
    pub summary: TestSummary,
    pub red_team_results: Vec<RedTeamResult>,
    pub multiplayer_determinism_passed: bool,
    pub multiplayer_desync_detected: bool,
    pub racer_scenario: RacerScenarioResult,
    pub arena_scenario: ArenaScenarioResult,
    pub frame_budget: FrameBudgetResult,
    pub chaos_results: Vec<ChaosResult>,
    pub engine_evidence: Vec<EngineEvidenceEntry>,
}

impl SyntheticReport {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Valid JSON serialization")
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# WorldVM Synthetic Validation Report\n\n");
        md.push_str(&format!("**Version**: {}  \n", self.version));
        md.push_str(&format!("**Timestamp**: {}  \n", self.execution_timestamp));
        md.push_str(&format!("**Profile**: `{}`  \n\n", self.profile));

        md.push_str("## Executive Summary\n\n");
        md.push_str(&format!("* **Total Tests Executed**: {}\n", self.summary.total_tests));
        md.push_str(&format!("* **Passed**: {}\n", self.summary.passed_tests));
        md.push_str(&format!("* **Failed**: {}\n", self.summary.failed_tests));
        md.push_str(&format!("* **Pass Rate**: {:.1}%\n", self.summary.pass_rate_percent));
        md.push_str(&format!("* **Duration**: {} ms\n\n", self.summary.execution_time_ms));

        md.push_str("## Red Team Security Scenarios (S001 – S015)\n\n");
        md.push_str("| ID | Attack Scenario | Status | Details |\n");
        md.push_str("|---|---|---|---|\n");
        for r in &self.red_team_results {
            let status = if r.passed { "PASS" } else { "FAIL" };
            md.push_str(&format!("| {} | {} | {} | {} |\n", r.scenario_id, r.name, status, r.details.replace('|', "\\|")));
        }
        md.push_str("\n");

        md.push_str("## Multiplayer Determinism & Desync\n\n");
        md.push_str(&format!("* **Deterministic Lockstep Hash Consistency**: {}\n", if self.multiplayer_determinism_passed { "PASS (Identical state hash across runs)" } else { "FAIL" }));
        md.push_str(&format!("* **Desync Detection on Divergent Inputs**: {}\n\n", if self.multiplayer_desync_detected { "PASS (Divergence cleanly detected)" } else { "FAIL" }));

        md.push_str("## Game Simulation Scenarios\n\n");
        md.push_str(&format!("* **Neon Racer**: {} (Laps: {}, Ticks: {}, XP: {}, Winner: `{}`)\n",
            if self.racer_scenario.passed { "PASS" } else { "FAIL" },
            self.racer_scenario.completed_laps,
            self.racer_scenario.total_ticks,
            self.racer_scenario.total_xp_awarded,
            self.racer_scenario.winner_id,
        ));
        md.push_str(&format!("* **Arena Zombie Survival**: {} (Spawned: {}, Damage Events: {}, Survivors: {}/4, Score: {})\n\n",
            if self.arena_scenario.passed { "PASS" } else { "FAIL" },
            self.arena_scenario.zombies_spawned,
            self.arena_scenario.player_damage_events,
            self.arena_scenario.players_survived,
            self.arena_scenario.total_score,
        ));

        md.push_str("## Frame Budget Benchmarks\n\n");
        md.push_str("| Target Cadence | Target Frame Time | Avg Execution Time | P99 Latency | Budget Exceeded |\n");
        md.push_str("|---|---|---|---|---|\n");
        let b30 = &self.frame_budget.stats_30hz;
        md.push_str(&format!("| 30 Hz | {} us | {:.1} us | {} us | {} |\n", b30.target_frame_time_us, b30.avg_tick_us, b30.p99_tick_us, b30.budget_exceeded_count));
        let b60 = &self.frame_budget.stats_60hz;
        md.push_str(&format!("| 60 Hz | {} us | {:.1} us | {} us | {} |\n", b60.target_frame_time_us, b60.avg_tick_us, b60.p99_tick_us, b60.budget_exceeded_count));
        let b120 = &self.frame_budget.stats_120hz;
        md.push_str(&format!("| 120 Hz | {} us | {:.1} us | {} us | {} |\n\n", b120.target_frame_time_us, b120.avg_tick_us, b120.p99_tick_us, b120.budget_exceeded_count));

        md.push_str("## Chaos & Fault Injection\n\n");
        md.push_str("| Scenario | Status | Details |\n");
        md.push_str("|---|---|---|\n");
        for c in &self.chaos_results {
            let status = if c.passed { "PASS" } else { "FAIL" };
            md.push_str(&format!("| {} | {} | {} |\n", c.name, status, c.details));
        }
        md.push_str("\n");

        md.push_str("## Engine Evidence Classification\n\n");
        md.push_str("| Engine | Evidence Class | Test Target | Status | Notes |\n");
        md.push_str("|---|---|---|---|---|\n");
        for e in &self.engine_evidence {
            md.push_str(&format!("| {} | {} | {} | {} | {} |\n", e.engine, e.classification, e.test_target, e.status, e.notes));
        }
        md.push_str("\n");

        md
    }

    pub fn save_artifacts(&self, json_path: &str, md_path: &str) -> std::io::Result<()> {
        if let Some(parent) = Path::new(json_path).parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = Path::new(md_path).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(json_path, self.to_json())?;
        fs::write(md_path, self.to_markdown())?;
        Ok(())
    }
}
