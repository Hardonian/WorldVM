using UnityEngine;

namespace WorldVM
{
    public class WorldVMBehaviour : MonoBehaviour
    {
        [Header("Configuration")]
        public TextAsset capabilityContract;
        public bool isServer = false;

        private void Awake()
        {
            string yaml = capabilityContract != null ? capabilityContract.text : null;
            WorldVMRuntime.Initialize(yaml, isServer);
        }

        private void OnDestroy()
        {
            WorldVMRuntime.Shutdown();
        }
    }
}
