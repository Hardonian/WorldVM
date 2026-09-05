//! WorldCapabilityContract, permission matrix, rate limits, and security evaluation.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use worldvm_core::WorldVmError;

/// High-level permission categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionCategory {
    Read,
    Write,
    Destructive,
    Economy,
    Communication,
    Network,
    Storage,
    Server,
    Custom,
}

/// Access level granted to a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAccess {
    Deny,
    Read,
    Write,
    Admin,
}

/// Execution location constraint for the capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityLocation {
    Client,
    Server,
    Both,
}

impl Default for CapabilityLocation {
    fn default() -> Self {
        Self::Both
    }
}

/// Rate limiting and value validation rules.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimitRule {
    #[serde(default)]
    pub calls_per_tick: Option<u32>,
    #[serde(default)]
    pub max_value: Option<u64>,
    #[serde(default)]
    pub allowed_types: Option<Vec<String>>,
}

/// Declaration for an individual capability in the host contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDefinition {
    pub access: CapabilityAccess,
    #[serde(default = "default_category")]
    pub category: PermissionCategory,
    #[serde(default)]
    pub location: CapabilityLocation,
    #[serde(default)]
    pub rate_limit: Option<RateLimitRule>,
}

fn default_category() -> PermissionCategory {
    PermissionCategory::Read
}

/// Formal WorldCapabilityContract defining what a host game exposes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldCapabilityContract {
    #[serde(rename = "apiVersion", default = "default_api_version")]
    pub api_version: String,
    pub game: GameInfo,
    #[serde(default)]
    pub capabilities: HashMap<String, CapabilityDefinition>,
}

fn default_api_version() -> String {
    "worldvm.dev/v1".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameInfo {
    pub id: String,
    #[serde(default)]
    pub version: Option<String>,
}

impl WorldCapabilityContract {
    pub fn from_yaml(content: &str) -> Result<Self, WorldVmError> {
        serde_yaml::from_str(content).map_err(|e| WorldVmError::InvalidPackage {
            reason: format!("Failed to parse CapabilityContract YAML: {e}"),
        })
    }

    pub fn from_toml(content: &str) -> Result<Self, WorldVmError> {
        toml::from_str(content).map_err(|e| WorldVmError::InvalidPackage {
            reason: format!("Failed to parse CapabilityContract TOML: {e}"),
        })
    }

    /// Standard baseline contract for generic arcade/arena games.
    pub fn standard_arcade_contract(game_id: &str) -> Self {
        let mut caps = HashMap::new();
        caps.insert(
            "player.read_position".to_string(),
            CapabilityDefinition {
                access: CapabilityAccess::Read,
                category: PermissionCategory::Read,
                location: CapabilityLocation::Both,
                rate_limit: Some(RateLimitRule {
                    calls_per_tick: Some(32),
                    ..Default::default()
                }),
            },
        );
        caps.insert(
            "world.set_gravity".to_string(),
            CapabilityDefinition {
                access: CapabilityAccess::Write,
                category: PermissionCategory::Write,
                location: CapabilityLocation::Both,
                rate_limit: Some(RateLimitRule {
                    calls_per_tick: Some(4),
                    ..Default::default()
                }),
            },
        );
        caps.insert(
            "world.get_gravity".to_string(),
            CapabilityDefinition {
                access: CapabilityAccess::Read,
                category: PermissionCategory::Read,
                location: CapabilityLocation::Both,
                rate_limit: None,
            },
        );
        caps.insert(
            "world.spawn".to_string(),
            CapabilityDefinition {
                access: CapabilityAccess::Write,
                category: PermissionCategory::Write,
                location: CapabilityLocation::Both,
                rate_limit: Some(RateLimitRule {
                    calls_per_tick: Some(10),
                    allowed_types: Some(vec![
                        "checkpoint".to_string(),
                        "zombie".to_string(),
                        "vehicle".to_string(),
                        "prop".to_string(),
                    ]),
                    ..Default::default()
                }),
            },
        );
        caps.insert(
            "ui.notify".to_string(),
            CapabilityDefinition {
                access: CapabilityAccess::Write,
                category: PermissionCategory::Communication,
                location: CapabilityLocation::Both,
                rate_limit: Some(RateLimitRule {
                    calls_per_tick: Some(8),
                    ..Default::default()
                }),
            },
        );
        caps.insert(
            "player.grant_xp".to_string(),
            CapabilityDefinition {
                access: CapabilityAccess::Write,
                category: PermissionCategory::Economy,
                location: CapabilityLocation::Server, // Economy defaults to server
                rate_limit: Some(RateLimitRule {
                    calls_per_tick: Some(2),
                    max_value: Some(5000),
                    ..Default::default()
                }),
            },
        );
        caps.insert(
            "network.http".to_string(),
            CapabilityDefinition {
                access: CapabilityAccess::Deny, // Denied by default
                category: PermissionCategory::Network,
                location: CapabilityLocation::Server,
                rate_limit: None,
            },
        );

        Self {
            api_version: "worldvm.dev/v1".to_string(),
            game: GameInfo {
                id: game_id.to_string(),
                version: Some("1.0.0".to_string()),
            },
            capabilities: caps,
        }
    }
}

/// Tracks runtime call quotas and enforces the capability contract.
#[derive(Debug, Clone)]
pub struct CapabilityEnforcer {
    contract: WorldCapabilityContract,
    granted_capabilities: HashSet<String>,
    current_tick: u64,
    tick_call_counts: HashMap<String, u32>,
    is_server_runtime: bool,
}

impl CapabilityEnforcer {
    pub fn new(
        contract: WorldCapabilityContract,
        requested_capabilities: &[String],
        is_server_runtime: bool,
    ) -> Self {
        let mut granted = HashSet::new();
        for cap in requested_capabilities {
            if let Some(def) = contract.capabilities.get(cap) {
                // Denied capabilities are never granted
                if def.access == CapabilityAccess::Deny {
                    continue;
                }
                // Server-only capabilities cannot be granted on client
                if !is_server_runtime && def.location == CapabilityLocation::Server {
                    continue;
                }
                granted.insert(cap.clone());
            }
        }

        Self {
            contract,
            granted_capabilities: granted,
            current_tick: 0,
            tick_call_counts: HashMap::new(),
            is_server_runtime,
        }
    }

    /// Advances the tick counter and resets per-tick call quotas.
    pub fn advance_tick(&mut self, tick: u64) {
        if self.current_tick != tick {
            self.current_tick = tick;
            self.tick_call_counts.clear();
        }
    }

    /// Verifies if a capability call is permitted and adheres to rate limits.
    pub fn check_call(&mut self, capability: &str) -> Result<(), WorldVmError> {
        // 1. Check if capability is granted to module
        if !self.granted_capabilities.contains(capability) {
            let reason = match self.contract.capabilities.get(capability) {
                Some(def) if def.access == CapabilityAccess::Deny => {
                    "Capability is explicitly denied by host contract".to_string()
                }
                Some(def) if !self.is_server_runtime && def.location == CapabilityLocation::Server => {
                    "Capability is restricted to server-authoritative runtime".to_string()
                }
                Some(_) => "Capability was not requested in module manifest".to_string(),
                None => "Capability is not exposed by host contract".to_string(),
            };
            return Err(WorldVmError::PermissionDenied {
                capability: capability.to_string(),
                reason,
            });
        }

        // 2. Check rate limit
        if let Some(def) = self.contract.capabilities.get(capability) {
            if let Some(ref rl) = def.rate_limit {
                if let Some(max_calls) = rl.calls_per_tick {
                    let count = self.tick_call_counts.entry(capability.to_string()).or_insert(0);
                    if *count >= max_calls {
                        return Err(WorldVmError::RateLimitExceeded {
                            capability: capability.to_string(),
                            limit: max_calls,
                        });
                    }
                    *count += 1;
                }
            }
        }

        Ok(())
    }

    pub fn is_granted(&self, capability: &str) -> bool {
        self.granted_capabilities.contains(capability)
    }

    pub fn granted_capabilities(&self) -> &HashSet<String> {
        &self.granted_capabilities
    }

    pub fn contract(&self) -> &WorldCapabilityContract {
        &self.contract
    }
}
