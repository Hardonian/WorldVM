using System;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;

namespace WorldVM
{
    public static class WorldVMRuntime
    {
        private const string LibName = "worldvm_c_api";

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate int CapabilityCallback(
            [MarshalAs(UnmanagedType.LPStr)] string moduleId,
            [MarshalAs(UnmanagedType.LPStr)] string capability,
            IntPtr inData,
            UIntPtr inLen,
            out IntPtr outData,
            out UIntPtr outLen,
            IntPtr userData
        );

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr worldvm_runtime_create(string contractYaml, int isServer);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern void worldvm_runtime_destroy(IntPtr runtime);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern void worldvm_register_capability_callback(IntPtr runtime, CapabilityCallback cb, IntPtr userData);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int worldvm_module_load(IntPtr runtime, byte[] packageBytes, UIntPtr packageLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern int worldvm_emit_event(IntPtr runtime, string moduleId, string eventName, byte[] payload, UIntPtr payloadLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr worldvm_last_error(IntPtr runtime);

        private static IntPtr _runtimeHandle;
        private static CapabilityCallback _cachedCallback;

        public static bool Initialize(string contractYaml = null, bool isServer = false)
        {
            if (_runtimeHandle != IntPtr.Zero) return true;

            _runtimeHandle = worldvm_runtime_create(contractYaml, isServer ? 1 : 0);
            if (_runtimeHandle == IntPtr.Zero)
            {
                Debug.LogError("[WorldVM] Failed to initialize native runtime.");
                return false;
            }

            _cachedCallback = OnNativeCapabilityCall;
            worldvm_register_capability_callback(_runtimeHandle, _cachedCallback, IntPtr.Zero);
            Debug.Log("[WorldVM] Runtime successfully initialized.");
            return true;
        }

        public static bool LoadModule(byte[] packageBytes)
        {
            if (_runtimeHandle == IntPtr.Zero) return false;
            int res = worldvm_module_load(_runtimeHandle, packageBytes, (UIntPtr)packageBytes.Length);
            return res == 0;
        }

        public static bool EmitEvent(string moduleId, string eventName, string jsonPayload)
        {
            if (_runtimeHandle == IntPtr.Zero) return false;
            byte[] bytes = Encoding.UTF8.GetBytes(jsonPayload);
            int res = worldvm_emit_event(_runtimeHandle, moduleId, eventName, bytes, (UIntPtr)bytes.Length);
            return res == 0;
        }

        private static int OnNativeCapabilityCall(
            string moduleId,
            string capability,
            IntPtr inData,
            UIntPtr inLen,
            out IntPtr outData,
            out UIntPtr outLen,
            IntPtr userData)
        {
            outData = IntPtr.Zero;
            outLen = UIntPtr.Zero;

            Debug.Log($"[WorldVM Host] Module '{moduleId}' invoked capability: '{capability}'");

            if (capability == "world.set_gravity")
            {
                Physics.gravity = new Vector3(0, -2.4f, 0);
                return 0; // Success
            }

            return -2; // Permission denied for unknown
        }

        public static void Shutdown()
        {
            if (_runtimeHandle != IntPtr.Zero)
            {
                worldvm_runtime_destroy(_runtimeHandle);
                _runtimeHandle = IntPtr.Zero;
            }
        }
    }
}
