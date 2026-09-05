//! Headless server-authoritative module execution engine and signed receipt generator.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_core::{ExecutionReceipt, WorldVmError};
use worldvm_package::WorldModPackage;
use worldvm_runtime::{WorldCapabilityProvider, WorldVmRuntime};
use worldvm_signing::sign_content;

/// Resource class determining credit pricing tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceClass {
    Micro,
    Standard,
    Heavy,
}

impl ResourceClass {
    pub fn classify(memory_mb: u32, fuel: u64) -> Self {
        if memory_mb <= 2 && fuel <= 100_000 {
            Self::Micro
        } else if memory_mb <= 32 && fuel <= 1_000_000 {
            Self::Standard
        } else {
            Self::Heavy
        }
    }

    pub fn base_credits(&self) -> u64 {
        match self {
            Self::Micro => 1,
            Self::Standard => 5,
            Self::Heavy => 20,
        }
    }
}

/// Headless server execution coordinator.
pub struct ServerExecutionEngine {
    runtime: WorldVmRuntime,
    game_id: String,
    signing_key: Option<ed25519_dalek::SigningKey>,
}

impl ServerExecutionEngine {
    pub fn new(
        game_id: impl Into<String>,
        contract: WorldCapabilityContract,
        provider: Arc<dyn WorldCapabilityProvider>,
        signing_key: Option<ed25519_dalek::SigningKey>,
    ) -> Result<Self, WorldVmError> {
        let runtime = WorldVmRuntime::new(contract, provider, true)?;
        Ok(Self {
            runtime,
            game_id: game_id.into(),
            signing_key,
        })
    }

    pub fn load_package(&mut self, package: WorldModPackage) -> Result<(), WorldVmError> {
        self.runtime.load_module(package)
    }

    pub fn execute_event(
        &mut self,
        module_id: &str,
        event_name: &str,
        payload: &[u8],
    ) -> Result<ExecutionReceipt, WorldVmError> {
        let execution_id = format!("exec_{}_{}", module_id, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos());
        
        let metrics = self.runtime.emit_event(module_id, event_name, payload)?;

        // Calculate result hash
        let mut hasher = Sha256::new();
        hasher.update(execution_id.as_bytes());
        hasher.update(module_id.as_bytes());
        hasher.update(event_name.as_bytes());
        hasher.update(&metrics.fuel_consumed.to_le_bytes());
        let result_hash = hex::encode(hasher.finalize());

        let res_class = ResourceClass::classify(32, metrics.fuel_consumed);
        let credits = res_class.base_credits();

        let mut receipt = ExecutionReceipt {
            execution_id,
            game_id: self.game_id.clone(),
            module_id: module_id.to_string(),
            module_hash: "canonical-hash".to_string(),
            module_version: "1.0.0".to_string(),
            event_name: event_name.to_string(),
            fuel_consumed: metrics.fuel_consumed,
            execution_time_us: metrics.execution_time_us,
            credits_charged: credits,
            result_hash: result_hash.clone(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            signature: None,
        };

        // Sign receipt if key provided
        if let Some(ref sk) = self.signing_key {
            let sig = sign_content(sk, &result_hash);
            receipt.signature = Some(sig.signature);
        }

        Ok(receipt)
    }
}
