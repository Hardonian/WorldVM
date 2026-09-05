//! C ABI implementation for WorldVM.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::Arc;
use parking_lot::RwLock;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_core::{ExecutionContext, WorldVmError};
use worldvm_package::WorldModPackage;
use worldvm_runtime::{WorldCapabilityProvider, WorldVmRuntime};

type CCapabilityCallback = unsafe extern "C" fn(
    module_id: *const c_char,
    capability: *const c_char,
    in_data: *const u8,
    in_len: usize,
    out_data: *mut *mut u8,
    out_len: *mut usize,
    user_data: *mut c_void,
) -> c_int;

struct CCallbackProvider {
    callback: RwLock<Option<(CCapabilityCallback, *mut c_void)>>,
}

unsafe impl Send for CCallbackProvider {}
unsafe impl Sync for CCallbackProvider {}

impl WorldCapabilityProvider for CCallbackProvider {
    fn call(
        &self,
        ctx: &ExecutionContext,
        capability: &str,
        input: &[u8],
    ) -> Result<Vec<u8>, WorldVmError> {
        let lock = self.callback.read();
        if let Some((cb, user_data)) = *lock {
            let c_mod = CString::new(ctx.module_id.as_str()).unwrap_or_default();
            let c_cap = CString::new(capability).unwrap_or_default();
            let mut out_ptr: *mut u8 = std::ptr::null_mut();
            let mut out_len: usize = 0;

            let code = unsafe {
                cb(
                    c_mod.as_ptr(),
                    c_cap.as_ptr(),
                    input.as_ptr(),
                    input.len(),
                    &mut out_ptr,
                    &mut out_len,
                    user_data,
                )
            };

            if code == 0 {
                if out_len > 0 && !out_ptr.is_null() {
                    let slice = unsafe { std::slice::from_raw_parts(out_ptr, out_len) };
                    let data = slice.to_vec();
                    // Free C allocated memory if needed
                    unsafe { worldvm_free_buffer(out_ptr, out_len) };
                    Ok(data)
                } else {
                    Ok(Vec::new())
                }
            } else if code == -2 {
                Err(WorldVmError::PermissionDenied {
                    capability: capability.to_string(),
                    reason: "C callback denied capability".to_string(),
                })
            } else {
                Err(WorldVmError::HostError {
                    message: format!("C callback returned error code {code}"),
                })
            }
        } else {
            Err(WorldVmError::CapabilityUnavailable {
                capability: capability.to_string(),
            })
        }
    }
}

pub struct WorldVmRuntimeHandle {
    runtime: WorldVmRuntime,
    provider: Arc<CCallbackProvider>,
    last_error: RwLock<Option<CString>>,
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_runtime_create(
    contract_yaml: *const c_char,
    is_server: c_int,
) -> *mut WorldVmRuntimeHandle {
    let contract = if !contract_yaml.is_null() {
        match CStr::from_ptr(contract_yaml).to_str() {
            Ok(s) => WorldCapabilityContract::from_yaml(s)
                .unwrap_or_else(|_| WorldCapabilityContract::standard_arcade_contract("default-game")),
            Err(_) => WorldCapabilityContract::standard_arcade_contract("default-game"),
        }
    } else {
        WorldCapabilityContract::standard_arcade_contract("default-game")
    };

    let provider = Arc::new(CCallbackProvider {
        callback: RwLock::new(None),
    });

    match WorldVmRuntime::new(contract, provider.clone(), is_server != 0) {
        Ok(runtime) => {
            let handle = Box::new(WorldVmRuntimeHandle {
                runtime,
                provider,
                last_error: RwLock::new(None),
            });
            Box::into_raw(handle)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_runtime_destroy(runtime: *mut WorldVmRuntimeHandle) {
    if !runtime.is_null() {
        let _ = Box::from_raw(runtime);
    }
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_register_capability_callback(
    runtime: *mut WorldVmRuntimeHandle,
    callback: CCapabilityCallback,
    user_data: *mut c_void,
) {
    if let Some(handle) = runtime.as_ref() {
        *handle.provider.callback.write() = Some((callback, user_data));
    }
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_module_load(
    runtime: *mut WorldVmRuntimeHandle,
    package_bytes: *const u8,
    package_len: usize,
) -> c_int {
    if runtime.is_null() || package_bytes.is_null() || package_len == 0 {
        return -1;
    }

    let handle = &mut *runtime;
    let bytes_slice = std::slice::from_raw_parts(package_bytes, package_len);

    let pkg = match WorldModPackage::from_bytes(bytes_slice) {
        Ok(p) => p,
        Err(e) => {
            *handle.last_error.write() = CString::new(e.to_string()).ok();
            return -4; // Invalid package
        }
    };

    match handle.runtime.load_module(pkg) {
        Ok(_) => 0,
        Err(e) => {
            *handle.last_error.write() = CString::new(e.to_string()).ok();
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_module_unload(
    runtime: *mut WorldVmRuntimeHandle,
    module_id: *const c_char,
) -> c_int {
    if runtime.is_null() || module_id.is_null() {
        return -1;
    }
    let handle = &mut *runtime;
    if let Ok(mid) = CStr::from_ptr(module_id).to_str() {
        if handle.runtime.unload_module(mid) {
            0
        } else {
            -5
        }
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_emit_event(
    runtime: *mut WorldVmRuntimeHandle,
    module_id: *const c_char,
    event_name: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> c_int {
    if runtime.is_null() || module_id.is_null() || event_name.is_null() {
        return -1;
    }

    let handle = &mut *runtime;
    let mid = match CStr::from_ptr(module_id).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let ev = match CStr::from_ptr(event_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let p_slice = if payload.is_null() || payload_len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(payload, payload_len)
    };

    match handle.runtime.emit_event(mid, ev, p_slice) {
        Ok(_) => 0,
        Err(WorldVmError::PermissionDenied { .. }) => -2,
        Err(WorldVmError::OutOfFuel { .. }) => -3,
        Err(e) => {
            *handle.last_error.write() = CString::new(e.to_string()).ok();
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_last_error(runtime: *mut WorldVmRuntimeHandle) -> *const c_char {
    if let Some(handle) = runtime.as_ref() {
        if let Some(ref err) = *handle.last_error.read() {
            return err.as_ptr();
        }
    }
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_module_fuel_consumed(
    runtime: *mut WorldVmRuntimeHandle,
    module_id: *const c_char,
) -> u64 {
    if let Some(handle) = runtime.as_ref() {
        if let Ok(mid) = CStr::from_ptr(module_id).to_str() {
            return handle
                .runtime
                .get_metrics(mid)
                .map(|m| m.fuel_consumed)
                .unwrap_or(0);
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn worldvm_free_buffer(buffer: *mut u8, len: usize) {
    if !buffer.is_null() && len > 0 {
        let _ = Vec::from_raw_parts(buffer, len, len);
    }
}
