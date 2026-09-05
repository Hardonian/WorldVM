//! Deterministic local simulated game host for WorldVM.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use worldvm_abi::*;
use worldvm_core::{ExecutionContext, WorldVmError};
use worldvm_runtime::WorldCapabilityProvider;

#[derive(Debug, Clone)]
pub struct MockPlayer {
    pub id: String,
    pub name: String,
    pub position: (f32, f32, f32),
    pub xp: u64,
    pub notifications: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MockEntity {
    pub id: u64,
    pub entity_type: String,
    pub position: (f32, f32, f32),
}

/// Simulated in-memory game state.
pub struct SimulatedWorldState {
    pub gravity: f32,
    pub players: HashMap<String, MockPlayer>,
    pub entities: HashMap<u64, MockEntity>,
    pub next_entity_id: AtomicU64,
    pub tick_count: u64,
    pub capability_history: Vec<(String, String)>,
}

impl Default for SimulatedWorldState {
    fn default() -> Self {
        let mut players = HashMap::new();
        players.insert(
            "player_1".to_string(),
            MockPlayer {
                id: "player_1".to_string(),
                name: "NeonRacer".to_string(),
                position: (0.0, 1.0, 0.0),
                xp: 100,
                notifications: Vec::new(),
            },
        );

        Self {
            gravity: 9.81, // Default Earth gravity
            players,
            entities: HashMap::new(),
            next_entity_id: AtomicU64::new(100),
            tick_count: 0,
            capability_history: Vec::new(),
        }
    }
}

/// A deterministic local game host simulating game engines.
#[derive(Clone)]
pub struct MockGameHost {
    state: Arc<RwLock<SimulatedWorldState>>,
}

impl Default for MockGameHost {
    fn default() -> Self {
        Self::new()
    }
}

impl MockGameHost {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(SimulatedWorldState::default())),
        }
    }

    pub fn get_gravity(&self) -> f32 {
        self.state.read().gravity
    }

    pub fn set_gravity(&self, gravity: f32) {
        self.state.write().gravity = gravity;
    }

    pub fn add_player(&self, id: &str, name: &str, pos: (f32, f32, f32)) {
        self.state.write().players.insert(
            id.to_string(),
            MockPlayer {
                id: id.to_string(),
                name: name.to_string(),
                position: pos,
                xp: 0,
                notifications: Vec::new(),
            },
        );
    }

    pub fn get_player(&self, id: &str) -> Option<MockPlayer> {
        self.state.read().players.get(id).cloned()
    }

    pub fn get_notifications(&self, player_id: &str) -> Vec<String> {
        self.state
            .read()
            .players
            .get(player_id)
            .map(|p| p.notifications.clone())
            .unwrap_or_default()
    }

    pub fn get_spawned_entities(&self) -> Vec<MockEntity> {
        self.state.read().entities.values().cloned().collect()
    }

    pub fn get_capability_history(&self) -> Vec<(String, String)> {
        self.state.read().capability_history.clone()
    }

    pub fn step_tick(&self, _delta_sec: f32) {
        self.state.write().tick_count += 1;
    }
}

impl WorldCapabilityProvider for MockGameHost {
    fn call(
        &self,
        ctx: &ExecutionContext,
        capability: &str,
        input: &[u8],
    ) -> Result<Vec<u8>, WorldVmError> {
        let mut state = self.state.write();
        state
            .capability_history
            .push((ctx.module_id.clone(), capability.to_string()));

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
                let entity_id = state.next_entity_id.fetch_add(1, Ordering::SeqCst);
                state.entities.insert(
                    entity_id,
                    MockEntity {
                        id: entity_id,
                        entity_type: parsed.entity_type,
                        position: (parsed.x, parsed.y, parsed.z),
                    },
                );
                serialize_payload(&SpawnEntityOutput { entity_id })
            }
            "world.despawn" => {
                let parsed: DespawnEntityInput = deserialize_payload(input)?;
                state.entities.remove(&parsed.entity_id);
                serialize_payload(&EmptyPayload {})
            }
            "player.read_position" => {
                let parsed: ReadPositionInput = deserialize_payload(input)?;
                let player = state
                    .players
                    .get(&parsed.player_id)
                    .ok_or_else(|| WorldVmError::HostError {
                        message: format!("Player '{}' not found", parsed.player_id),
                    })?;
                serialize_payload(&Vector3Output {
                    x: player.position.0,
                    y: player.position.1,
                    z: player.position.2,
                })
            }
            "player.grant_xp" => {
                let parsed: GrantXpInput = deserialize_payload(input)?;
                let player = state
                    .players
                    .get_mut(&parsed.player_id)
                    .ok_or_else(|| WorldVmError::HostError {
                        message: format!("Player '{}' not found", parsed.player_id),
                    })?;
                player.xp += parsed.amount;
                serialize_payload(&GrantXpOutput { new_xp: player.xp })
            }
            "ui.notify" => {
                let parsed: NotifyPlayerInput = deserialize_payload(input)?;
                if let Some(player) = state.players.get_mut(&parsed.player_id) {
                    player.notifications.push(parsed.message.clone());
                }
                serialize_payload(&EmptyPayload {})
            }
            "network.http" => {
                // If permission was granted (e.g. in custom test setup), return mock HTTP response
                serialize_payload(&HttpFetchOutput {
                    status: 200,
                    body: "{\"status\":\"ok\"}".to_string(),
                })
            }
            _ => Err(WorldVmError::CapabilityUnavailable {
                capability: capability.to_string(),
            }),
        }
    }
}
