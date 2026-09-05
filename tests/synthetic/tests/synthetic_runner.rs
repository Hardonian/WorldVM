//! Integration test runner for cargo test -p tests-synthetic

use tests_synthetic::run_synthetic_suite;

#[test]
fn test_synthetic_lab_full_suite() {
    let report = run_synthetic_suite("ci", 1337);

    assert_eq!(
        report.summary.failed_tests, 0,
        "Expected zero synthetic test failures, but {} tests failed",
        report.summary.failed_tests
    );

    assert!(report.multiplayer_determinism_passed, "Multiplayer determinism failed");
    assert!(report.multiplayer_desync_detected, "Multiplayer desync detection failed");
    assert!(report.racer_scenario.passed, "Racer scenario failed");
    assert!(report.arena_scenario.passed, "Arena scenario failed");
    assert!(report.frame_budget.passed, "Frame budget validation failed");

    for red_team in &report.red_team_results {
        assert!(
            red_team.passed,
            "Red team scenario {} ({}) failed: {}",
            red_team.scenario_id, red_team.name, red_team.details
        );
    }

    for chaos in &report.chaos_results {
        assert!(
            chaos.passed,
            "Chaos test {} failed: {}",
            chaos.name, chaos.details
        );
    }
}
