//! SyntheticCapabilityProvider & Audit Call Logger.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use worldvm_abi::*;
use worldvm_core::{ExecutionContext, WorldVmError};
use worldvm_runtime::WorldCapabilityProvider;

use super::state::{SimEntity, WorldState};

#[derive(Debug, Clone)]
pub struct RecordedHostCall {
    pub module_id: String,
    pub capability: String,
    pub input_hash: String,
    pub result: String,
    pub tick: u64,
}

pub struct SyntheticCapabilityProvider {
    pub state: Arc<RwLock<WorldState>>,
    pub audit_log: Arc<RwLock<Vec<RecordedHostCall>>>,
    pub next_entity_id: AtomicU64,
}

impl SyntheticCapabilityProvider {
    pub fn new(state: Arc<RwLock<WorldState>>) -> Self {
        Self {
            state,
            audit_log: Arc::new(RwLock::new(Vec::new())),
            next_entity_id: AtomicU64::new(100),
        }
    }

    pub fn get_audit_log(&self) -> Vec<RecordedHostCall> {
        self.audit_log.read().clone()
    }
}

impl WorldCapabilityProvider for SyntheticCapabilityProvider {
    fn call(
        &self,
        ctx: &ExecutionContext,
        capability: &str,
        input: &[u8],
    ) -> Result<Vec<u8>, WorldVmError> {
        let mut hasher = Sha256::new();
        hasher.update(input);
        let input_hash = hex::encode(&hasher.finalize()[..8]);

        let current_tick;
        let result: Result<Vec<u8>, WorldVmError> = {
            let mut state = self.state.write();
            current_tick = state.tick;

            match capability {
                // --- WORLD CAPABILITIES ---
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
                    let entity_id = self.next_entity_id.fetch_add(1, Ordering::SeqCst);
                    state.entities.insert(
                        entity_id,
                        SimEntity {
                            id: entity_id,
                            entity_type: parsed.entity_type,
                            x: parsed.x,
                            y: parsed.y,
                            z: parsed.z,
                            health: 100.0,
                            is_alive: true,
                        },
                    );
                    serialize_payload(&SpawnEntityOutput { entity_id })
                }
                "world.despawn" => {
                    let parsed: DespawnEntityInput = deserialize_payload(input)?;
                    if state.entities.remove(&parsed.entity_id).is_some() {
                        serialize_payload(&EmptyPayload {})
                    } else {
                        Err(WorldVmError::EntityNotFound {
                            entity_id: parsed.entity_id,
                        })
                    }
                }

                // --- PLAYER CAPABILITIES ---
                "player.read_position" => {
                    let parsed: ReadPositionInput = deserialize_payload(input)?;
                    if let Some(player) = state.players.get(&parsed.player_id) {
                        serialize_payload(&Vector3Output {
                            x: player.x,
                            y: player.y,
                            z: player.z,
                        })
                    } else {
                        Err(WorldVmError::HostError {
                            message: format!("Player '{}' not found", parsed.player_id),
                        })
                    }
                }
                "player.grant_xp" => {
                    let parsed: GrantXpInput = deserialize_payload(input)?;
                    if let Some(player) = state.players.get_mut(&parsed.player_id) {
                        player.xp += parsed.amount;
                        serialize_payload(&GrantXpOutput { new_xp: player.xp })
                    } else {
                        Err(WorldVmError::HostError {
                            message: format!("Player '{}' not found", parsed.player_id),
                        })
                    }
                }
                "player.apply_damage" => {
                    #[derive(serde::Deserialize)]
                    struct DamageInput {
                        player_id: String,
                        amount: f32,
                    }
                    #[derive(serde::Serialize)]
                    struct DamageOutput {
                        remaining_health: f32,
                        is_dead: bool,
                    }

                    let parsed: DamageInput = deserialize_payload(input)?;
                    if let Some(player) = state.players.get_mut(&parsed.player_id) {
                        player.health = (player.health - parsed.amount).max(0.0);
                        let is_dead = player.health == 0.0;
                        serialize_payload(&DamageOutput {
                            remaining_health: player.health,
                            is_dead,
                        })
                    } else {
                        Err(WorldVmError::HostError {
                            message: format!("Player '{}' not found", parsed.player_id),
                        })
                    }
                }

                // --- UI CAPABILITIES ---
                "ui.notify" => {
                    let parsed: NotifyPlayerInput = deserialize_payload(input)?;
                    if let Some(player) = state.players.get_mut(&parsed.player_id) {
                        player.notifications.push(parsed.message);
                    }
                    serialize_payload(&EmptyPayload {})
                }

                // --- INVENTORY CAPABILITIES ---
                "inventory.read" => {
                    #[derive(serde::Deserialize)]
                    struct InvInput {
                        player_id: String,
                    }
                    let parsed: InvInput = deserialize_payload(input)?;
                    if let Some(player) = state.players.get(&parsed.player_id) {
                        serialize_payload(&player.inventory)
                    } else {
                        Err(WorldVmError::HostError {
                            message: format!("Player '{}' not found", parsed.player_id),
                        })
                    }
                }
                "inventory.grant" => {
                    #[derive(serde::Deserialize)]
                    struct GrantInput {
                        player_id: String,
                        item_id: String,
                        quantity: u32,
                    }
                    let parsed: GrantInput = deserialize_payload(input)?;
                    if let Some(player) = state.players.get_mut(&parsed.player_id) {
                        let count = player.inventory.entry(parsed.item_id).or_insert(0);
                        *count += parsed.quantity;
                        serialize_payload(&EmptyPayload {})
                    } else {
                        Err(WorldVmError::HostError {
                            message: format!("Player '{}' not found", parsed.player_id),
                        })
                    }
                }

                // --- STORAGE CAPABILITIES ---
                "storage.set" => {
                    #[derive(serde::Deserialize)]
                    struct SetInput {
                        key: String,
                        val: String,
                    }
                    let parsed: SetInput = deserialize_payload(input)?;
                    // Namespace strictly per module
                    let namespaced_key = format!("{}:{}", ctx.module_id, parsed.key);
                    state.persistent_storage.insert(namespaced_key, parsed.val);
                    serialize_payload(&EmptyPayload {})
                }
                "storage.get" => {
                    #[derive(serde::Deserialize)]
                    struct GetInput {
                        key: String,
                    }
                    let parsed: GetInput = deserialize_payload(input)?;
                    let namespaced_key = format!("{}:{}", ctx.module_id, parsed.key);
                    let val = state.persistent_storage.get(&namespaced_key).cloned().unwrap_or_default();
                    serialize_payload(&val)
                }

                // --- NETWORK (MOCK) ---
                "network.http" => {
                    let parsed: HttpFetchInput = deserialize_payload(input)?;
                    // SSRF Protection: Deny private IP space
                    if parsed.url.contains("localhost")
                        || parsed.url.contains("127.0.0.1")
                        || parsed.url.contains("169.254.")
                        || parsed.url.contains("10.")
                        || parsed.url.contains("192.168.")
                    {
                        return Err(WorldVmError::PermissionDenied {
                            capability: "network.http".to_string(),
                            reason: "SSRF Attempt blocked: internal address forbidden".to_string(),
                        });
                    }
                    serialize_payload(&HttpFetchOutput {
                        status: 200,
                        body: "{\"status\":\"simulated_ok\"}".to_string(),
                    })
                }

                _ => Err(WorldVmError::CapabilityUnavailable {
                    capability: capability.to_string(),
                }),
            }
        };

        // Record in audit log
        let res_desc = match &result {
            Ok(_) => "OK".to_string(),
            Err(e) => format!("ERR: {e}"),
        };

        self.audit_log.write().push(RecordedHostCall {
            module_id: ctx.module_id.clone(),
            capability: capability.to_string(),
            input_hash,
            result: res_desc,
            tick: current_tick,
        });

        result
    }
}
