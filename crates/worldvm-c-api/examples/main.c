#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../include/worldvm.h"

static int my_engine_capabilities(
    const char* module_id,
    const char* capability,
    const uint8_t* in_data,
    size_t in_len,
    uint8_t** out_data,
    size_t* out_len,
    void* user_data
) {
    printf("[Custom C Engine] Module '%s' called capability: %s\n", module_id, capability);

    if (strcmp(capability, "world.set_gravity") == 0) {
        printf("[Custom C Engine] Setting world gravity!\n");
        *out_data = NULL;
        *out_len = 0;
        return WORLDVM_OK;
    }

    if (strcmp(capability, "ui.notify") == 0) {
        printf("[Custom C Engine] Notification toast displayed.\n");
        *out_data = NULL;
        *out_len = 0;
        return WORLDVM_OK;
    }

    /* Deny unknown */
    return WORLDVM_ERR_PERMISSION_DENIED;
}

int main(int argc, char** argv) {
    printf("=========================================\n");
    printf(" WorldVM Embedded C Engine Host Example \n");
    printf("=========================================\n");

    /* Initialize runtime */
    worldvm_runtime_t* runtime = worldvm_runtime_create(NULL, 0);
    if (!runtime) {
        fprintf(stderr, "Failed to initialize WorldVM runtime.\n");
        return 1;
    }
    printf("WorldVM runtime initialized successfully.\n");

    /* Register game engine capability callback */
    worldvm_register_capability_callback(runtime, my_engine_capabilities, NULL);
    printf("Registered engine capability provider.\n");

    /* Emit a test host event */
    const char* test_event = "round_start";
    const uint8_t payload[] = "{\"match_id\":\"arena-01\",\"round_number\":1}";
    int res = worldvm_emit_event(runtime, "test-mod", test_event, payload, sizeof(payload) - 1);
    printf("Emitted event '%s' (result: %d)\n", test_event, res);

    /* Clean up */
    worldvm_runtime_destroy(runtime);
    printf("Runtime destroyed. Done.\n");

    return 0;
}
