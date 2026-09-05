use worldvm_capabilities::{
    CapabilityAccess, CapabilityEnforcer, WorldCapabilityContract,
};
use worldvm_core::WorldVmError;

#[test]
fn test_contract_yaml_parsing() {
    let yaml = r#"
apiVersion: worldvm.dev/v1
game:
  id: neon-drift
  version: 1.0.0
capabilities:
  player.read_position:
    access: read
    category: read
    location: both
    rate_limit:
      calls_per_tick: 4
  world.set_gravity:
    access: write
    category: write
    location: both
  network.http:
    access: deny
    category: network
    location: server
"#;

    let contract = WorldCapabilityContract::from_yaml(yaml).expect("Valid YAML");
    assert_eq!(contract.game.id, "neon-drift");
    assert_eq!(contract.capabilities.len(), 3);

    let http_cap = contract.capabilities.get("network.http").unwrap();
    assert_eq!(http_cap.access, CapabilityAccess::Deny);
}

#[test]
fn test_permission_enforcement_and_denial() {
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");

    // Module requests: world.set_gravity (allowed), network.http (denied by contract), filesystem.read (undeclared)
    let requested = vec![
        "world.set_gravity".to_string(),
        "network.http".to_string(),
        "filesystem.read".to_string(),
    ];

    let mut enforcer = CapabilityEnforcer::new(contract, &requested, false);

    // 1. world.set_gravity should be granted
    assert!(enforcer.is_granted("world.set_gravity"));
    assert!(enforcer.check_call("world.set_gravity").is_ok());

    // 2. network.http is denied by contract
    assert!(!enforcer.is_granted("network.http"));
    let res = enforcer.check_call("network.http");
    assert!(matches!(res, Err(WorldVmError::PermissionDenied { .. })));

    // 3. filesystem.read is not exposed by host
    assert!(!enforcer.is_granted("filesystem.read"));
    let res = enforcer.check_call("filesystem.read");
    assert!(matches!(res, Err(WorldVmError::PermissionDenied { .. })));
}

#[test]
fn test_server_only_capability_denied_on_client() {
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");
    let requested = vec!["player.grant_xp".to_string()];

    // On client runtime (is_server = false)
    let client_enforcer = CapabilityEnforcer::new(contract.clone(), &requested, false);
    assert!(!client_enforcer.is_granted("player.grant_xp"));

    // On server runtime (is_server = true)
    let server_enforcer = CapabilityEnforcer::new(contract, &requested, true);
    assert!(server_enforcer.is_granted("player.grant_xp"));
}

#[test]
fn test_rate_limiting_per_tick() {
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");
    // world.set_gravity has a rate limit of 4 calls per tick
    let requested = vec!["world.set_gravity".to_string()];
    let mut enforcer = CapabilityEnforcer::new(contract, &requested, false);

    // First 4 calls succeed
    for _ in 0..4 {
        assert!(enforcer.check_call("world.set_gravity").is_ok());
    }

    // 5th call exceeds rate limit
    let fifth = enforcer.check_call("world.set_gravity");
    assert!(matches!(fifth, Err(WorldVmError::RateLimitExceeded { limit: 4, .. })));

    // Advance tick -> resets quota
    enforcer.advance_tick(1);
    assert!(enforcer.check_call("world.set_gravity").is_ok());
}
