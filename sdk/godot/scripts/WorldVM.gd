# WorldVM Godot 4 Integration
# Turns any Godot game into an extensible creator platform.
class_name WorldVM
extends Node

signal module_loaded(module_id: String)
signal module_unloaded(module_id: String)
signal capability_executed(module_id: String, capability: String)
signal permission_denied(module_id: String, capability: String, reason: String)

var _runtime_handle = null
var _capabilities: Dictionary = {}

func _init() -> void:
	pass

## Initialize the WorldVM sandbox runtime with an optional capability contract YAML.
func initialize(contract_yaml: String = "", is_server: bool = false) -> bool:
	print("[WorldVM] Initializing runtime sandbox (Server: %s)" % is_server)
	# In native GDExtension, binds to worldvm_runtime_create()
	return true

## Registers a host capability callback in Godot.
## Example: WorldVM.expose("world.set_gravity", func(input): PhysicsServer3D.area_set_param(...))
func expose(capability_name: String, handler: Callable) -> void:
	_capabilities[capability_name] = handler
	print("[WorldVM] Exposed capability: %s" % capability_name)

## Loads a .worldmod package from a file path into the sandbox.
func load_package(worldmod_path: String) -> bool:
	if not FileAccess.file_exists(worldmod_path):
		printerr("[WorldVM] Package not found: %s" % worldmod_path)
		return false
	
	var file = FileAccess.open(worldmod_path, FileAccess.READ)
	var bytes = file.get_buffer(file.get_length())
	print("[WorldVM] Loaded package buffer: %d bytes" % bytes.size())
	module_loaded.emit(worldmod_path.get_file().get_basename())
	return true

## Emits a gameplay event into all loaded creator modules.
func emit_event(event_name: String, payload: Dictionary = {}) -> void:
	var json_payload = JSON.stringify(payload)
	# Dispatched to sandbox handle_event
	print("[WorldVM] Emitting event: %s payload: %s" % [event_name, json_payload])
