//! Core types, entity handles, error models, and execution metrics for WorldVM.

use std::fmt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current WorldVM version.
pub const WORLDVM_VERSION: &str = "1.0.0";

/// Current WorldVM ABI version.
pub const WORLDVM_ABI_VERSION: &str = "1.0";

/// Opaque player identifier. No PII is exposed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub String);

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for PlayerId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for PlayerId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Opaque entity identifier for host world objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "entity#{}", self.0)
    }
}

/// Opaque match/session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MatchId(pub String);

impl fmt::Display for MatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Opaque item identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub String);

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Capability identifier (e.g. "player.read_position", "world.spawn").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub String);

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for CapabilityId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for CapabilityId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Comprehensive WorldVM error taxonomy.
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorldVmError {
    #[error("Permission denied for capability '{capability}': {reason}")]
    PermissionDenied {
        capability: String,
        reason: String,
    },

    #[error("Execution exhausted fuel budget: limit {fuel_limit}, consumed {consumed}")]
    OutOfFuel {
        fuel_limit: u64,
        consumed: u64,
    },

    #[error("Execution deadline exceeded: timeout was {timeout_ms} ms")]
    DeadlineExceeded {
        timeout_ms: u64,
    },

    #[error("Memory allocation limit exceeded: limit {limit_bytes} bytes, requested {requested_bytes} bytes")]
    MemoryLimitExceeded {
        limit_bytes: usize,
        requested_bytes: usize,
    },

    #[error("Invalid .worldmod package: {reason}")]
    InvalidPackage {
        reason: String,
    },

    #[error("Invalid or missing package signature: {reason}")]
    InvalidSignature {
        reason: String,
    },

    #[error("ABI mismatch: expected {expected}, found {found}")]
    AbiMismatch {
        expected: String,
        found: String,
    },

    #[error("Capability unavailable on this host: {capability}")]
    CapabilityUnavailable {
        capability: String,
    },

    #[error("Host error during capability execution: {message}")]
    HostError {
        message: String,
    },

    #[error("WebAssembly trap occurred [{trap_code}]: {message}")]
    ModuleTrap {
        trap_code: String,
        message: String,
    },

    #[error("Reentrancy disallowed for module '{module}'")]
    ReentrancyDisallowed {
        module: String,
    },

    #[error("Serialization / Deserialization error: {message}")]
    SerializationError {
        message: String,
    },

    #[error("Entity not found: entity #{entity_id}")]
    EntityNotFound {
        entity_id: u64,
    },

    #[error("Rate limit exceeded for capability '{capability}': limit was {limit} calls")]
    RateLimitExceeded {
        capability: String,
        limit: u32,
    },

    #[error("Module not loaded: '{module_id}'")]
    ModuleNotLoaded {
        module_id: String,
    },

    #[error("Module circuit breaker tripped: module '{module_id}' disabled due to consecutive failures")]
    CircuitBreakerTripped {
        module_id: String,
    },
}

/// Resource constraints enforced per module instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum linear memory in megabytes (e.g. 32 MB).
    pub memory_mb: u32,
    /// Maximum WebAssembly instruction fuel (e.g. 500,000 instructions).
    pub fuel_limit: u64,
    /// Maximum execution deadline in milliseconds per event (e.g. 5 ms).
    pub max_execution_ms: u64,
    /// Maximum event invocations allowed per tick.
    pub max_events_per_tick: u32,
    /// Maximum nested capability / event call depth.
    pub max_call_depth: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: 32,
            fuel_limit: 500_000,
            max_execution_ms: 5,
            max_events_per_tick: 64,
            max_call_depth: 8,
        }
    }
}

/// Execution mode for a module invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Frame-bound game loop tick.
    Tick,
    /// Event triggered by host or player action.
    Event,
    /// Direct query/request from game host.
    Request,
    /// Long-running asynchronous creator job outside frame budget.
    Job,
}

/// Runtime telemetry captured per invocation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub invocations: u64,
    pub fuel_consumed: u64,
    pub execution_time_us: u64,
    pub host_calls: u64,
    pub memory_high_water_mark_bytes: usize,
    pub errors_encountered: u64,
}

/// Verifiable cryptographic execution receipt for headless / server execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub execution_id: String,
    pub game_id: String,
    pub module_id: String,
    pub module_hash: String,
    pub module_version: String,
    pub event_name: String,
    pub fuel_consumed: u64,
    pub execution_time_us: u64,
    pub credits_charged: u64,
    pub result_hash: String,
    pub timestamp: u64,
    pub signature: Option<String>,
}

/// Execution context passed into host capability providers.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub module_id: String,
    pub publisher: String,
    pub mode: ExecutionMode,
    pub tick: u64,
    pub delta_seconds: f32,
}
