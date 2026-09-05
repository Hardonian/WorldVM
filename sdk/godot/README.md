# WorldVM Godot 4 SDK

Embed the WorldVM WebAssembly creator sandbox inside Godot 4.x games using GDExtension and GDScript.

## Features

- Run sandboxed untrusted mods with strict instruction fuel budgets.
- Expose engine capabilities directly from GDScript via `WorldVM.expose()`.
- Dispatch gameplay events to creator mods (`player_join`, `round_start`, etc.).
- Prevent malicious filesystem, network, or OS access.

## Quickstart

```gdscript
extends Node

@onready var worldvm = WorldVM.new()

func _ready():
    # 1. Initialize runtime
    worldvm.initialize()

    # 2. Expose safe engine capabilities to creator mods
    worldvm.expose("world.set_gravity", func(input):
        var gravity = input.get("gravity", 9.81)
        PhysicsServer3D.area_set_param(get_viewport().find_world_3d().space, PhysicsServer3D.AREA_PARAM_GRAVITY, gravity)
    )

    # 3. Load a creator mod
    worldvm.load_package("res://mods/low-gravity.worldmod")

    # 4. Emit gameplay events
    worldvm.emit_event("round_start", { "round": 1 })
```
