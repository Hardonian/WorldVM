# WorldVM Unity Package (UPM)

Sandboxed WebAssembly gameplay execution runtime for Unity games.

## Installation

Add the package to your `Packages/manifest.json`:

```json
{
  "dependencies": {
    "dev.worldvm.unity": "file:../../sdk/unity"
  }
}
```

## Usage

```csharp
using UnityEngine;
using WorldVM;

public class GameController : MonoBehaviour
{
    void Start()
    {
        WorldVMRuntime.Initialize();
        
        // Load creator .worldmod package
        byte[] packageBytes = System.IO.File.ReadAllBytes("Assets/Mods/low-gravity.worldmod");
        WorldVMRuntime.LoadModule(packageBytes);

        // Emit match event
        WorldVMRuntime.EmitEvent("low-gravity", "round_start", "{}");
    }
}
```
