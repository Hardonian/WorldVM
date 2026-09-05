//! Creator Rust SDK for WorldVM. Provides safe, typed host capability APIs and test mock utilities.

use std::alloc::{alloc, dealloc, Layout};
use worldvm_abi::*;
use worldvm_core::WorldVmError;

pub use worldvm_abi as abi;
pub use worldvm_core as core;

// ----------------------------------------------------------------------------
// Host Call Binding (WASM vs Host Mock)
// ----------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "worldvm_env")]
extern "C" {
    fn worldvm_host_call(
        cap_ptr: *const u8,
        cap_len: usize,
        in_ptr: *const u8,
        in_len: usize,
        out_ptr_ptr: *mut *mut u8,
        out_len_ptr: *mut usize,
    ) -> i32;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod test_mock {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static RECORDED_CALLS: RefCell<Vec<(String, Vec<u8>)>> = RefCell::new(Vec::new());
        static MOCK_RESPONSES: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new());
    }

    pub fn reset() {
        RECORDED_CALLS.with(|c| c.borrow_mut().clear());
        MOCK_RESPONSES.with(|r| r.borrow_mut().clear());
    }

    pub fn record_call(capability: &str, input: &[u8]) {
        RECORDED_CALLS.with(|c| c.borrow_mut().push((capability.to_string(), input.to_vec())));
    }

    pub fn get_calls() -> Vec<(String, Vec<u8>)> {
        RECORDED_CALLS.with(|c| c.borrow().clone())
    }

    pub fn set_response(capability: &str, response: Vec<u8>) {
        MOCK_RESPONSES.with(|r| r.borrow_mut().insert(capability.to_string(), response));
    }

    pub fn get_response(capability: &str) -> Option<Vec<u8>> {
        MOCK_RESPONSES.with(|r| r.borrow().get(capability).cloned())
    }

    pub fn assert_called(capability: &str) {
        let calls = get_calls();
        assert!(
            calls.iter().any(|(c, _)| c == capability),
            "Expected capability '{}' to have been called",
            capability
        );
    }
}

/// Invokes a host capability across the sandbox boundary.
pub fn call_host(capability: &str, input: &[u8]) -> Result<Vec<u8>, WorldVmError> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut out_ptr: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        let ret = unsafe {
            worldvm_host_call(
                capability.as_ptr(),
                capability.len(),
                input.as_ptr(),
                input.len(),
                &mut out_ptr as *mut *mut u8,
                &mut out_len as *mut usize,
            )
        };

        match ret {
            ABI_SUCCESS => {
                if out_len == 0 || out_ptr.is_null() {
                    Ok(Vec::new())
                } else {
                    let slice = unsafe { std::slice::from_raw_parts(out_ptr, out_len) };
                    let output = slice.to_vec();
                    // Free allocated guest memory
                    guest_free_internal(out_ptr, out_len);
                    Ok(output)
                }
            }
            ABI_ERR_PERMISSION_DENIED => Err(WorldVmError::PermissionDenied {
                capability: capability.to_string(),
                reason: "Host rejected capability execution".to_string(),
            }),
            ABI_ERR_CAPABILITY_NOT_FOUND => Err(WorldVmError::CapabilityUnavailable {
                capability: capability.to_string(),
            }),
            _ => Err(WorldVmError::HostError {
                message: format!("Host call to '{capability}' failed with code {ret}"),
            }),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        test_mock::record_call(capability, input);
        if let Some(resp) = test_mock::get_response(capability) {
            Ok(resp)
        } else {
            Ok(Vec::new())
        }
    }
}

// ----------------------------------------------------------------------------
// Typed Capability APIs
// ----------------------------------------------------------------------------

pub mod world {
    use super::*;

    /// Sets game world gravity in m/s² (standard Earth gravity is 9.81).
    pub fn set_gravity(gravity: f32) -> Result<(), WorldVmError> {
        let payload = serialize_payload(&SetGravityInput { gravity })?;
        call_host("world.set_gravity", &payload)?;
        Ok(())
    }

    /// Gets current world gravity.
    pub fn get_gravity() -> Result<f32, WorldVmError> {
        let resp = call_host("world.get_gravity", &[])?;
        let out: GetGravityOutput = deserialize_payload(&resp)?;
        Ok(out.gravity)
    }

    /// Spawns an entity of given type at (x, y, z).
    pub fn spawn(entity_type: &str, x: f32, y: f32, z: f32) -> Result<u64, WorldVmError> {
        let payload = serialize_payload(&SpawnEntityInput {
            entity_type: entity_type.to_string(),
            x,
            y,
            z,
        })?;
        let resp = call_host("world.spawn", &payload)?;
        let out: SpawnEntityOutput = deserialize_payload(&resp)?;
        Ok(out.entity_id)
    }

    /// Despawns an entity.
    pub fn despawn(entity_id: u64) -> Result<(), WorldVmError> {
        let payload = serialize_payload(&DespawnEntityInput { entity_id })?;
        call_host("world.despawn", &payload)?;
        Ok(())
    }
}

pub mod player {
    use super::*;

    /// Reads player position (x, y, z).
    pub fn read_position(player_id: &str) -> Result<(f32, f32, f32), WorldVmError> {
        let payload = serialize_payload(&ReadPositionInput {
            player_id: player_id.to_string(),
        })?;
        let resp = call_host("player.read_position", &payload)?;
        let out: Vector3Output = deserialize_payload(&resp)?;
        Ok((out.x, out.y, out.z))
    }

    /// Grants XP to a player (typically server-authoritative).
    pub fn grant_xp(player_id: &str, amount: u64) -> Result<u64, WorldVmError> {
        let payload = serialize_payload(&GrantXpInput {
            player_id: player_id.to_string(),
            amount,
        })?;
        let resp = call_host("player.grant_xp", &payload)?;
        let out: GrantXpOutput = deserialize_payload(&resp)?;
        Ok(out.new_xp)
    }
}

pub mod ui {
    use super::*;

    /// Displays an on-screen toast / banner notification to a player.
    pub fn notify(player_id: &str, message: &str, duration_seconds: f32) -> Result<(), WorldVmError> {
        let payload = serialize_payload(&NotifyPlayerInput {
            player_id: player_id.to_string(),
            message: message.to_string(),
            duration_seconds,
        })?;
        call_host("ui.notify", &payload)?;
        Ok(())
    }
}

pub mod network {
    use super::*;

    /// Makes an HTTP request (only granted if host explicitly exposes network.http).
    pub fn fetch(url: &str, method: &str, body: Option<&str>) -> Result<(u16, String), WorldVmError> {
        let payload = serialize_payload(&HttpFetchInput {
            url: url.to_string(),
            method: method.to_string(),
            body: body.map(|s| s.to_string()),
        })?;
        let resp = call_host("network.http", &payload)?;
        let out: HttpFetchOutput = deserialize_payload(&resp)?;
        Ok((out.status, out.body))
    }
}

// ----------------------------------------------------------------------------
// Guest Memory Allocation Handlers
// ----------------------------------------------------------------------------

pub fn guest_alloc_internal(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size, 8).unwrap_or(Layout::new::<u8>());
    unsafe { alloc(layout) }
}

pub fn guest_free_internal(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        let layout = Layout::from_size_align(size, 8).unwrap_or(Layout::new::<u8>());
        unsafe { dealloc(ptr, layout) }
    }
}

// ----------------------------------------------------------------------------
// Entrypoint Export Macro
// ----------------------------------------------------------------------------

/// Macro to export all mandatory WorldVM guest ABI symbols from creator crate.
#[macro_export]
macro_rules! export_entrypoint {
    ($handler_fn:expr) => {
        #[no_mangle]
        pub extern "C" fn worldvm_get_abi_version() -> u32 {
            $crate::abi::ABI_VERSION_V1
        }

        #[no_mangle]
        pub extern "C" fn worldvm_guest_alloc(size: usize) -> *mut u8 {
            $crate::guest_alloc_internal(size)
        }

        #[no_mangle]
        pub extern "C" fn worldvm_guest_free(ptr: *mut u8, size: usize) {
            $crate::guest_free_internal(ptr, size)
        }

        #[no_mangle]
        pub extern "C" fn worldvm_handle_event(
            name_ptr: *const u8,
            name_len: usize,
            payload_ptr: *const u8,
            payload_len: usize,
        ) -> i32 {
            if name_ptr.is_null() {
                return $crate::abi::ABI_ERR_INVALID_PAYLOAD;
            }

            let name_slice = unsafe { std::slice::from_raw_parts(name_ptr, name_len) };
            let event_name = match std::str::from_utf8(name_slice) {
                Ok(s) => s,
                Err(_) => return $crate::abi::ABI_ERR_INVALID_PAYLOAD,
            };

            let payload_slice = if payload_ptr.is_null() || payload_len == 0 {
                &[]
            } else {
                unsafe { std::slice::from_raw_parts(payload_ptr, payload_len) }
            };

            $handler_fn(event_name, payload_slice);

            $crate::abi::ABI_SUCCESS
        }
    };
}

pub mod prelude {
    pub use super::abi::*;
    pub use super::core::*;
    pub use super::{network, player, ui, world, export_entrypoint};
}
