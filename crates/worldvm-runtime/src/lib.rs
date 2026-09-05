//! WebAssembly sandbox engine, capability enforcement, fuel metering, and lifecycle management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};
use wasmtime::{
    Caller, Config, Engine, Instance, Linker, Module, Store, TypedFunc,
};
use worldvm_abi::{
    guest_exports, host_imports, ABI_ERR_CAPABILITY_NOT_FOUND, ABI_ERR_GENERIC,
    ABI_ERR_INVALID_PAYLOAD, ABI_ERR_PERMISSION_DENIED, ABI_SUCCESS,
};
use worldvm_capabilities::{CapabilityEnforcer, WorldCapabilityContract};
use worldvm_core::{
    ExecutionContext, ExecutionMetrics, ExecutionMode, ResourceLimits, WorldVmError,
};
use worldvm_package::WorldModPackage;

/// Trait for host game engines to receive and handle capability invocations.
pub trait WorldCapabilityProvider: Send + Sync {
    fn call(
        &self,
        ctx: &ExecutionContext,
        capability: &str,
        input: &[u8],
    ) -> Result<Vec<u8>, WorldVmError>;
}

/// Fallback no-op capability provider.
pub struct DefaultCapabilityProvider;
impl WorldCapabilityProvider for DefaultCapabilityProvider {
    fn call(
        &self,
        _ctx: &ExecutionContext,
        capability: &str,
        _input: &[u8],
    ) -> Result<Vec<u8>, WorldVmError> {
        Err(WorldVmError::CapabilityUnavailable {
            capability: capability.to_string(),
        })
    }
}

/// Internal store data associated with each module's Wasmtime Store.
pub struct StoreData {
    pub module_id: String,
    pub publisher: String,
    pub enforcer: CapabilityEnforcer,
    pub metrics: ExecutionMetrics,
    pub provider: Arc<dyn WorldCapabilityProvider>,
    pub execution_ctx: ExecutionContext,
    pub last_denial: Option<String>,
    pub consecutive_failures: u32,
    pub is_disabled: bool,
}

/// A loaded module instance ready for event execution.
pub struct LoadedModule {
    pub package: WorldModPackage,
    store: Store<StoreData>,
    instance: Instance,
    alloc_fn: TypedFunc<i32, i32>,
    free_fn: TypedFunc<(i32, i32), ()>,
    handle_event_fn: TypedFunc<(i32, i32, i32, i32), i32>,
    pub limits: ResourceLimits,
}

/// The primary WorldVM runtime managing sandbox instances.
pub struct WorldVmRuntime {
    engine: Engine,
    linker: Linker<StoreData>,
    modules: HashMap<String, LoadedModule>,
    contract: WorldCapabilityContract,
    provider: Arc<dyn WorldCapabilityProvider>,
    is_server: bool,
    sentinel: worldvm_sentinel::AdaptiveThreatDetector,
}

impl WorldVmRuntime {
    /// Creates a new WorldVM sandbox runtime instance with fuel metering and epoch interruption.
    pub fn new(
        contract: WorldCapabilityContract,
        provider: Arc<dyn WorldCapabilityProvider>,
        is_server: bool,
    ) -> Result<Self, WorldVmError> {
        let mut config = Config::new();
        // 1. Instruction fuel metering for deterministic limits
        config.consume_fuel(true);
        // 2. Epoch interruption for wall-clock execution deadlines
        config.epoch_interruption(true);

        let engine = Engine::new(&config).map_err(|e| WorldVmError::HostError {
            message: format!("Failed to create Wasmtime engine: {e}"),
        })?;

        let mut linker = Linker::new(&engine);

        // Define host function: worldvm_env::worldvm_host_call
        linker
            .func_wrap(
                host_imports::MODULE_NAME,
                host_imports::HOST_CALL,
                move |mut caller: Caller<'_, StoreData>,
                      cap_ptr: i32,
                      cap_len: i32,
                      in_ptr: i32,
                      in_len: i32,
                      out_ptr_ptr: i32,
                      out_len_ptr: i32|
                      -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(mem)) => mem,
                        _ => return ABI_ERR_GENERIC,
                    };

                    // Extract capability name
                    let mem_slice = memory.data(&caller);
                    let cap_start = cap_ptr as usize;
                    let cap_end = cap_start + cap_len as usize;
                    if cap_end > mem_slice.len() {
                        return ABI_ERR_INVALID_PAYLOAD;
                    }
                    let cap_name = match std::str::from_utf8(&mem_slice[cap_start..cap_end]) {
                        Ok(s) => s.to_string(),
                        Err(_) => return ABI_ERR_INVALID_PAYLOAD,
                    };

                    // Extract input bytes
                    let in_start = in_ptr as usize;
                    let in_end = in_start + in_len as usize;
                    if in_end > mem_slice.len() {
                        return ABI_ERR_INVALID_PAYLOAD;
                    }
                    let input_data = mem_slice[in_start..in_end].to_vec();

                    // Security check: Verify permission & rate limits
                    let check_res = caller.data_mut().enforcer.check_call(&cap_name);
                    if let Err(err) = check_res {
                        let denial_msg = format!("Capability '{cap_name}' denied: {err}");
                        warn!("{denial_msg}");
                        caller.data_mut().last_denial = Some(denial_msg);
                        caller.data_mut().metrics.errors_encountered += 1;
                        return ABI_ERR_PERMISSION_DENIED;
                    }

                    // Dispatch to host provider
                    let ctx = caller.data().execution_ctx.clone();
                    let provider = caller.data().provider.clone();
                    let call_result = provider.call(&ctx, &cap_name, &input_data);

                    caller.data_mut().metrics.host_calls += 1;

                    match call_result {
                        Ok(output_bytes) => {
                            if output_bytes.is_empty() {
                                return ABI_SUCCESS;
                            }

                            // Allocate guest buffer via exported worldvm_guest_alloc
                            let alloc_func = match caller.get_export(guest_exports::ALLOC) {
                                Some(wasmtime::Extern::Func(f)) => {
                                    match f.typed::<i32, i32>(&caller) {
                                        Ok(tf) => tf,
                                        Err(_) => return ABI_ERR_GENERIC,
                                    }
                                }
                                _ => return ABI_ERR_GENERIC,
                            };

                            let out_len = output_bytes.len() as i32;
                            let guest_out_ptr = match alloc_func.call(&mut caller, out_len) {
                                Ok(ptr) => ptr,
                                Err(e) => {
                                    error!("Failed to allocate guest buffer: {e}");
                                    return ABI_ERR_GENERIC;
                                }
                            };

                            // Write output bytes to allocated memory
                            let memory = match caller.get_export("memory") {
                                Some(wasmtime::Extern::Memory(mem)) => mem,
                                _ => return ABI_ERR_GENERIC,
                            };
                            let mem_mut = memory.data_mut(&mut caller);
                            let target_start = guest_out_ptr as usize;
                            let target_end = target_start + output_bytes.len();
                            if target_end > mem_mut.len() {
                                return ABI_ERR_GENERIC;
                            }
                            mem_mut[target_start..target_end].copy_from_slice(&output_bytes);

                            // Write pointer and length to out_ptr_ptr and out_len_ptr
                            if (out_ptr_ptr as usize + 4) <= mem_mut.len() {
                                mem_mut[out_ptr_ptr as usize..out_ptr_ptr as usize + 4]
                                    .copy_from_slice(&guest_out_ptr.to_le_bytes());
                            }
                            if (out_len_ptr as usize + 4) <= mem_mut.len() {
                                mem_mut[out_len_ptr as usize..out_len_ptr as usize + 4]
                                    .copy_from_slice(&out_len.to_le_bytes());
                            }

                            ABI_SUCCESS
                        }
                        Err(WorldVmError::CapabilityUnavailable { .. }) => {
                            ABI_ERR_CAPABILITY_NOT_FOUND
                        }
                        Err(WorldVmError::PermissionDenied { .. }) => {
                            ABI_ERR_PERMISSION_DENIED
                        }
                        Err(_) => ABI_ERR_GENERIC,
                    }
                },
            )
            .map_err(|e| WorldVmError::HostError {
                message: format!("Failed to link host function: {e}"),
            })?;

        Ok(Self {
            engine,
            linker,
            modules: HashMap::new(),
            contract,
            provider,
            is_server,
            sentinel: worldvm_sentinel::AdaptiveThreatDetector::new(),
        })
    }

    /// Loads, validates, and instantiates a .worldmod package inside an isolated sandbox.
    pub fn load_module(&mut self, package: WorldModPackage) -> Result<(), WorldVmError> {
        let module_id = package.manifest.name.clone();
        let publisher = package.manifest.publisher.clone();
        let limits: ResourceLimits = (&package.manifest.resources).into();

        // 1. Static validation of WASM binary
        let wasm_module = Module::new(&self.engine, &package.wasm_bytes).map_err(|e| {
            WorldVmError::InvalidPackage {
                reason: format!("WASM bytecode compilation failed: {e}"),
            }
        })?;

        // 2. Validate declared imports: reject any undeclared imports
        for import in wasm_module.imports() {
            let mod_name = import.module();
            if mod_name != host_imports::MODULE_NAME {
                return Err(WorldVmError::InvalidPackage {
                    reason: format!(
                        "Module imports forbidden or undeclared namespace: '{mod_name}'"
                    ),
                });
            }
        }

        // 3. Initialize CapabilityEnforcer for this module
        let enforcer = CapabilityEnforcer::new(
            self.contract.clone(),
            &package.manifest.permissions.request,
            self.is_server,
        );

        let execution_ctx = ExecutionContext {
            module_id: module_id.clone(),
            publisher: publisher.clone(),
            mode: ExecutionMode::Event,
            tick: 0,
            delta_seconds: 0.0,
        };

        let store_data = StoreData {
            module_id: module_id.clone(),
            publisher,
            enforcer,
            metrics: ExecutionMetrics::default(),
            provider: self.provider.clone(),
            execution_ctx,
            last_denial: None,
            consecutive_failures: 0,
            is_disabled: false,
        };

        let mut store = Store::new(&self.engine, store_data);
        store.set_epoch_deadline(1);
        store.set_fuel(limits.fuel_limit).map_err(|e| WorldVmError::HostError {
            message: format!("Failed to set initial fuel: {e}"),
        })?;

        // 4. Instantiate module
        let instance = self
            .linker
            .instantiate(&mut store, &wasm_module)
            .map_err(|e| WorldVmError::HostError {
                message: format!("Failed to instantiate module: {e}"),
            })?;

        // 5. Extract mandatory guest exports
        let alloc_fn = instance
            .get_typed_func::<i32, i32>(&mut store, guest_exports::ALLOC)
            .map_err(|_| WorldVmError::InvalidPackage {
                reason: format!("Missing required export '{}'", guest_exports::ALLOC),
            })?;

        let free_fn = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, guest_exports::FREE)
            .map_err(|_| WorldVmError::InvalidPackage {
                reason: format!("Missing required export '{}'", guest_exports::FREE),
            })?;

        let handle_event_fn = instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&mut store, guest_exports::HANDLE_EVENT)
            .map_err(|_| WorldVmError::InvalidPackage {
                reason: format!("Missing required export '{}'", guest_exports::HANDLE_EVENT),
            })?;

        info!("Successfully loaded module: {} v{}", package.manifest.name, package.manifest.version);

        self.modules.insert(
            module_id,
            LoadedModule {
                package,
                store,
                instance,
                alloc_fn,
                free_fn,
                handle_event_fn,
                limits,
            },
        );

        Ok(())
    }

    /// Emits an event to a specific loaded module.
    pub fn emit_event(
        &mut self,
        module_id: &str,
        event_name: &str,
        payload: &[u8],
    ) -> Result<ExecutionMetrics, WorldVmError> {
        let loaded = self
            .modules
            .get_mut(module_id)
            .ok_or_else(|| WorldVmError::ModuleNotLoaded {
                module_id: module_id.to_string(),
            })?;

        if loaded.store.data().is_disabled {
            return Err(WorldVmError::CircuitBreakerTripped {
                module_id: module_id.to_string(),
            });
        }

        // Reset fuel budget for this invocation
        let fuel_limit = loaded.limits.fuel_limit;
        let _ = loaded.store.set_fuel(fuel_limit);
        loaded.store.set_epoch_deadline(1);

        let start_time = Instant::now();

        // 1. Allocate event name in guest
        let name_bytes = event_name.as_bytes();
        let name_ptr = loaded
            .alloc_fn
            .call(&mut loaded.store, name_bytes.len() as i32)
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("fuel") {
                    WorldVmError::OutOfFuel {
                        fuel_limit,
                        consumed: fuel_limit,
                    }
                } else {
                    WorldVmError::ModuleTrap {
                        trap_code: "ALLOC_FAIL".to_string(),
                        message: format!("Allocation for event name failed: {e}"),
                    }
                }
            })?;

        // 2. Allocate payload in guest
        let payload_ptr = loaded
            .alloc_fn
            .call(&mut loaded.store, payload.len() as i32)
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("fuel") {
                    WorldVmError::OutOfFuel {
                        fuel_limit,
                        consumed: fuel_limit,
                    }
                } else {
                    WorldVmError::ModuleTrap {
                        trap_code: "ALLOC_FAIL".to_string(),
                        message: format!("Allocation for payload failed: {e}"),
                    }
                }
            })?;

        // 3. Write data to guest memory
        if let Some(wasmtime::Extern::Memory(mem)) = loaded.instance.get_export(&mut loaded.store, "memory") {
            let mem_mut = mem.data_mut(&mut loaded.store);
            let n_start = name_ptr as usize;
            let n_end = n_start + name_bytes.len();
            if n_end <= mem_mut.len() {
                mem_mut[n_start..n_end].copy_from_slice(name_bytes);
            }
            let p_start = payload_ptr as usize;
            let p_end = p_start + payload.len();
            if p_end <= mem_mut.len() {
                mem_mut[p_start..p_end].copy_from_slice(payload);
            }
        }

        // 4. Invoke guest handle_event function
        let invocation_result = loaded.handle_event_fn.call(
            &mut loaded.store,
            (
                name_ptr,
                name_bytes.len() as i32,
                payload_ptr,
                payload.len() as i32,
            ),
        );

        // 5. Clean up guest allocations
        let _ = loaded
            .free_fn
            .call(&mut loaded.store, (name_ptr, name_bytes.len() as i32));
        let _ = loaded
            .free_fn
            .call(&mut loaded.store, (payload_ptr, payload.len() as i32));

        let elapsed_us = start_time.elapsed().as_micros() as u64;
        let remaining_fuel = loaded.store.get_fuel().unwrap_or(0);
        let consumed_fuel = fuel_limit.saturating_sub(remaining_fuel);

        // Update metrics
        loaded.store.data_mut().metrics.invocations += 1;
        loaded.store.data_mut().metrics.fuel_consumed += consumed_fuel;
        loaded.store.data_mut().metrics.execution_time_us += elapsed_us;

        // Evaluate autonomous threat detector
        let had_denial = loaded.store.data().last_denial.is_some();
        let entropy = worldvm_sentinel::AdaptiveThreatDetector::calculate_entropy(payload);
        let host_calls = loaded.store.data().metrics.host_calls;
        let assessment = self.sentinel.evaluate(
            module_id,
            consumed_fuel,
            host_calls,
            had_denial,
            entropy,
        );

        if assessment.should_quarantine {
            warn!("Sentinel quarantine triggered for module '{}': {}", module_id, assessment.primary_indicator);
            loaded.store.data_mut().is_disabled = true;
        }

        // Check invocation trap
        match invocation_result {
            Ok(_ret_code) => {
                loaded.store.data_mut().consecutive_failures = 0;
                let mut m = loaded.store.data().metrics.clone();
                m.execution_time_us = elapsed_us;
                m.fuel_consumed = consumed_fuel;
                Ok(m)
            }
            Err(trap_err) => {
                loaded.store.data_mut().consecutive_failures += 1;
                loaded.store.data_mut().metrics.errors_encountered += 1;

                if loaded.store.data().consecutive_failures >= 3 {
                    warn!("Circuit breaker tripped for module '{}'", module_id);
                    loaded.store.data_mut().is_disabled = true;
                }

                // Discriminate out-of-fuel vs standard trap
                let debug_str = format!("{:?}", trap_err);
                let err_str = trap_err.to_string();
                if remaining_fuel == 0
                    || debug_str.contains("OutOfFuel")
                    || err_str.contains("fuel")
                    || err_str.contains("OutOfFuel")
                {
                    Err(WorldVmError::OutOfFuel {
                        fuel_limit,
                        consumed: fuel_limit,
                    })
                } else {
                    Err(WorldVmError::ModuleTrap {
                        trap_code: "EXEC_TRAP".to_string(),
                        message: err_str,
                    })
                }
            }
        }
    }

    /// Broadcasts an event to all loaded modules that subscribe to it.
    pub fn broadcast_event(
        &mut self,
        event_name: &str,
        payload: &[u8],
    ) -> HashMap<String, Result<ExecutionMetrics, WorldVmError>> {
        let mut results = HashMap::new();
        let module_ids: Vec<String> = self.modules.keys().cloned().collect();

        for mid in module_ids {
            let subscribes = self
                .modules
                .get(&mid)
                .map(|m| {
                    m.package.manifest.events.subscribe.is_empty()
                        || m.package.manifest.events.subscribe.iter().any(|e| e == event_name)
                })
                .unwrap_or(false);

            if subscribes {
                let res = self.emit_event(&mid, event_name, payload);
                results.insert(mid, res);
            }
        }

        results
    }

    /// Returns the active metrics for a given module.
    pub fn get_metrics(&self, module_id: &str) -> Option<ExecutionMetrics> {
        self.modules.get(module_id).map(|m| m.store.data().metrics.clone())
    }

    /// Returns the last permission denial message for a module if one occurred.
    pub fn get_last_denial(&self, module_id: &str) -> Option<String> {
        self.modules.get(module_id).and_then(|m| m.store.data().last_denial.clone())
    }

    /// Unloads a module from runtime.
    pub fn unload_module(&mut self, module_id: &str) -> bool {
        self.modules.remove(module_id).is_some()
    }

    /// Returns list of loaded module IDs.
    pub fn loaded_modules(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    /// Advances simulation tick across all active modules and resets per-tick rate limit counters.
    pub fn advance_tick(&mut self, tick: u64) {
        for loaded in self.modules.values_mut() {
            let data = loaded.store.data_mut();
            data.execution_ctx.tick = tick;
            data.enforcer.advance_tick(tick);
        }
    }

    /// Returns a reference to the runtime's adaptive threat detector.
    pub fn sentinel(&self) -> &worldvm_sentinel::AdaptiveThreatDetector {
        &self.sentinel
    }

    /// Retrieves behavioral profile metrics for a loaded module.
    pub fn get_behavioral_profile(&self, module_id: &str) -> Option<worldvm_sentinel::BehavioralProfile> {
        self.sentinel.get_profile(module_id)
    }
}

