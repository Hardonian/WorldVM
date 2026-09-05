//! WorldVM ABI v1: Host-guest interface specification, marshaling, and standard event schemas.

use serde::{Deserialize, Serialize};
use worldvm_core::WorldVmError;

/// Magic value returned by `worldvm_get_abi_version()` (Major: 1, Minor: 0 -> 0x0100).
pub const ABI_VERSION_V1: u32 = 0x0100;

/// Return codes across the C ABI boundary.
pub const ABI_SUCCESS: i32 = 0;
pub const ABI_ERR_GENERIC: i32 = -1;
pub const ABI_ERR_PERMISSION_DENIED: i32 = -2;
pub const ABI_ERR_OUT_OF_FUEL: i32 = -3;
pub const ABI_ERR_INVALID_PAYLOAD: i32 = -4;
pub const ABI_ERR_CAPABILITY_NOT_FOUND: i32 = -5;

/// Exported function names expected by WorldVM runtime on creator WASM modules.
pub mod guest_exports {
    pub const ALLOC: &str = "worldvm_guest_alloc";
    pub const FREE: &str = "worldvm_guest_free";
    pub const HANDLE_EVENT: &str = "worldvm_handle_event";
    pub const GET_ABI_VERSION: &str = "worldvm_get_abi_version";
}

/// Host function imports provided by WorldVM sandbox into creator modules.
pub mod host_imports {
    pub const MODULE_NAME: &str = "worldvm_env";
    pub const HOST_CALL: &str = "worldvm_host_call";
}

// ----------------------------------------------------------------------------
// Standard Host Event Payloads
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerJoinPayload {
    pub player_id: String,
    pub player_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLeavePayload {
    pub player_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerDeathPayload {
    pub player_id: String,
    pub killer_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundStartPayload {
    pub match_id: String,
    pub round_number: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundEndPayload {
    pub match_id: String,
    pub winner_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointPayload {
    pub player_id: String,
    pub checkpoint_id: u32,
    pub lap: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickPayload {
    pub delta_seconds: f32,
    pub tick_number: u64,
}

// ----------------------------------------------------------------------------
// Standard Host Capability Payloads
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmptyPayload {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetGravityInput {
    pub gravity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetGravityOutput {
    pub gravity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotifyPlayerInput {
    pub player_id: String,
    pub message: String,
    pub duration_seconds: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadPositionInput {
    pub player_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vector3Output {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnEntityInput {
    pub entity_type: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnEntityOutput {
    pub entity_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DespawnEntityInput {
    pub entity_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrantXpInput {
    pub player_id: String,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrantXpOutput {
    pub new_xp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpFetchInput {
    pub url: String,
    pub method: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpFetchOutput {
    pub status: u16,
    pub body: String,
}

// ----------------------------------------------------------------------------
// Serialization Helpers
// ----------------------------------------------------------------------------

pub fn serialize_payload<T: Serialize>(val: &T) -> Result<Vec<u8>, WorldVmError> {
    serde_json::to_vec(val).map_err(|e| WorldVmError::SerializationError {
        message: format!("Failed to serialize payload: {e}"),
    })
}

pub fn deserialize_payload<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, WorldVmError> {
    serde_json::from_slice(bytes).map_err(|e| WorldVmError::SerializationError {
        message: format!("Failed to deserialize payload: {e}"),
    })
}
