//! Synthetic Test Lab CLI executable.

use clap::Parser;
use colored::Colorize;
use tests_synthetic::run_synthetic_suite;

#[derive(Parser, Debug)]
#[command(name = "synthetic-lab")]
#[command(about = "WorldVM Autonomous Synthetic Test Lab Runner")]
struct Args {
    /// Test profile to execute (smoke, ci, hostile, soak, all)
    #[arg(short, long, default_value = "ci")]
    profile: String,

    /// Deterministic RNG seed for simulation scenarios
    #[arg(short, long, default_value_t = 42)]
    seed: u64,

    /// Output path for structured JSON validation report
    #[arg(long, default_value = "artifacts/synthetic-report.json")]
    out_json: String,

    /// Output path for executive Markdown validation report
    #[arg(long, default_value = "artifacts/synthetic-report.md")]
    out_md: String,
}

fn main() {
    let args = Args::parse();

    println!("{}", "=======================================================".bright_cyan());
    println!("{}", "  WORLDVM SYNTHETIC TEST LAB — FULL STACK VALIDATION  ".bold().bright_white());
    println!("{}", "=======================================================".bright_cyan());
    println!("Profile : {}", args.profile.bold().yellow());
    println!("Seed    : {}", args.seed.to_string().bold());
    println!();

    println!("{}", "Executing test matrix across red-team, multiplayer, games, frame budget, and chaos...".bright_blue());
    let report = run_synthetic_suite(&args.profile, args.seed);

    println!("\n{}", "--- 1. RED TEAM SECURITY MATRIX (S001 - S015) ---".bold().bright_magenta());
    for r in &report.red_team_results {
        let tag = if r.passed {
            "[PASS]".bold().green()
        } else {
            "[FAIL]".bold().red()
        };
        println!("  {} {} - {} ({})", tag, r.scenario_id.bright_yellow(), r.name, r.details.dimmed());
    }

    println!("\n{}", "--- 2. MULTIPLAYER CONSISTENCY ---".bold().bright_magenta());
    println!("  Lockstep Determinism : {}", if report.multiplayer_determinism_passed { "[PASS]".bold().green() } else { "[FAIL]".bold().red() });
    println!("  Desync Divergence    : {}", if report.multiplayer_desync_detected { "[PASS]".bold().green() } else { "[FAIL]".bold().red() });

    println!("\n{}", "--- 3. GAME SIMULATION SCENARIOS ---".bold().bright_magenta());
    println!("  Neon Racer           : {} ({})",
        if report.racer_scenario.passed { "[PASS]".bold().green() } else { "[FAIL]".bold().red() },
        report.racer_scenario.message.dimmed()
    );
    println!("  Arena Zombie Waves   : {} ({})",
        if report.arena_scenario.passed { "[PASS]".bold().green() } else { "[FAIL]".bold().red() },
        report.arena_scenario.message.dimmed()
    );

    println!("\n{}", "--- 4. FRAME BUDGET BENCHMARKS ---".bold().bright_magenta());
    let fb = &report.frame_budget;
    println!("  30 Hz  : avg={:.1}us, p99={}us, spikes={}", fb.stats_30hz.avg_tick_us, fb.stats_30hz.p99_tick_us, fb.stats_30hz.budget_exceeded_count);
    println!("  60 Hz  : avg={:.1}us, p99={}us, spikes={}", fb.stats_60hz.avg_tick_us, fb.stats_60hz.p99_tick_us, fb.stats_60hz.budget_exceeded_count);
    println!("  120 Hz : avg={:.1}us, p99={}us, spikes={}", fb.stats_120hz.avg_tick_us, fb.stats_120hz.p99_tick_us, fb.stats_120hz.budget_exceeded_count);
    println!("  Status : {}", if fb.passed { "[PASS]".bold().green() } else { "[FAIL]".bold().red() });

    println!("\n{}", "--- 5. CHAOS & FAULT INJECTION ---".bold().bright_magenta());
    for c in &report.chaos_results {
        let tag = if c.passed {
            "[PASS]".bold().green()
        } else {
            "[FAIL]".bold().red()
        };
        println!("  {} {} - {}", tag, c.name.bold(), c.details.dimmed());
    }

    println!("\n{}", "--- 6. ENGINE EVIDENCE MATRIX ---".bold().bright_magenta());
    for e in &report.engine_evidence {
        println!("  [{}] {} ({}) - {}", e.status.bold().green(), e.engine.bold().bright_cyan(), e.classification.bright_yellow(), e.notes.dimmed());
    }

    println!("\n{}", "=======================================================".bright_cyan());
    println!(
        "RESULTS: {}/{} tests passed ({:.1}%) in {} ms",
        report.summary.passed_tests.to_string().bold().green(),
        report.summary.total_tests.to_string().bold(),
        report.summary.pass_rate_percent,
        report.summary.execution_time_ms
    );
    println!("{}", "=======================================================".bright_cyan());

    // Save artifacts
    if let Err(e) = report.save_artifacts(&args.out_json, &args.out_md) {
        eprintln!("Warning: Failed to save artifacts: {e}");
    } else {
        println!("Artifacts written to:");
        println!("  JSON report : {}", args.out_json.bold().green());
        println!("  Markdown    : {}", args.out_md.bold().green());
    }

    // Also write to docs/SYNTHETIC_TEST_REPORT.md for repository documentation
    let _ = report.save_artifacts(&args.out_json, "docs/SYNTHETIC_TEST_REPORT.md");

    if report.summary.failed_tests > 0 {
        std::process::exit(1);
    }
}
