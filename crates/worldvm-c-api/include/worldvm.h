/**
 * WorldVM C ABI — Embeddable WebAssembly gameplay execution runtime.
 * Suitable for C, C++, Godot (GDExtension), Unity (P/Invoke), Unreal (C++ Plugin), and custom engines.
 */

#ifndef WORLDVM_H
#define WORLDVM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Return codes */
#define WORLDVM_OK 0
#define WORLDVM_ERR_GENERIC -1
#define WORLDVM_ERR_PERMISSION_DENIED -2
#define WORLDVM_ERR_OUT_OF_FUEL -3
#define WORLDVM_ERR_INVALID_PACKAGE -4
#define WORLDVM_ERR_NOT_FOUND -5

/* Opaque runtime handle */
typedef struct worldvm_runtime worldvm_runtime_t;

/**
 * Capability callback signature for host engine.
 * Receives capability invocations from creator modules.
 */
typedef int (*worldvm_capability_callback_t)(
    const char* module_id,
    const char* capability,
    const uint8_t* in_data,
    size_t in_len,
    uint8_t** out_data,
    size_t* out_len,
    void* user_data
);

/**
 * Creates a new WorldVM runtime instance with the specified capability contract YAML.
 * If contract_yaml is NULL, a default arcade contract is used.
 */
worldvm_runtime_t* worldvm_runtime_create(const char* contract_yaml, int is_server);

/**
 * Destroys a WorldVM runtime instance and frees all associated resources.
 */
void worldvm_runtime_destroy(worldvm_runtime_t* runtime);

/**
 * Registers the host engine's capability callback.
 */
void worldvm_register_capability_callback(
    worldvm_runtime_t* runtime,
    worldvm_capability_callback_t callback,
    void* user_data
);

/**
 * Loads a .worldmod package from memory into the sandbox.
 * Returns WORLDVM_OK on success or an error code.
 */
int worldvm_module_load(
    worldvm_runtime_t* runtime,
    const uint8_t* package_bytes,
    size_t package_len
);

/**
 * Unloads a module from the runtime.
 */
int worldvm_module_unload(
    worldvm_runtime_t* runtime,
    const char* module_id
);

/**
 * Emits an event to a specific loaded module.
 */
int worldvm_emit_event(
    worldvm_runtime_t* runtime,
    const char* module_id,
    const char* event_name,
    const uint8_t* payload,
    size_t payload_len
);

/**
 * Returns the last error message or NULL if no error occurred.
 */
const char* worldvm_last_error(worldvm_runtime_t* runtime);

/**
 * Returns total fuel consumed by a module.
 */
uint64_t worldvm_module_fuel_consumed(
    worldvm_runtime_t* runtime,
    const char* module_id
);

/**
 * Helper to free buffers allocated by capability callbacks.
 */
void worldvm_free_buffer(uint8_t* buffer, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* WORLDVM_H */
