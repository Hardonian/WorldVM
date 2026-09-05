# WorldVM Unreal Engine 5 Plugin

**Status**: `INTEGRATION_READY_UNVERIFIED`

This plugin exposes the WorldVM runtime to Unreal Engine 5 via C++ and Blueprints.

## Blueprint Nodes Exposed

- **Load WorldMod**: Loads a `.worldmod` package into the sandbox.
- **Unload WorldMod**: Safely unloads a creator module.
- **Emit WorldVM Event**: Dispatches typed events from Unreal into WebAssembly modules.
- **Register Capability**: Routes creator calls to Unreal gameplay subsystems.
