//! WorldVM Sentinel — Autonomous adaptive threat detection, behavioral anomaly engine, and tarpit shield.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Classification of current module threat posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatLevel {
    /// Normal baseline operations (Anomaly Score < 0.30)
    Normal,
    /// Elevated suspicion (0.30 <= Anomaly Score < 0.70). Tarpit backpressure engaged.
    Elevated,
    /// Critical threat detected (Anomaly Score >= 0.70). Instant quarantine & signature generated.
    Critical,
}

impl ThreatLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Elevated => "ELEVATED",
            Self::Critical => "CRITICAL",
        }
    }
}

/// Instantaneous threat assessment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAssessment {
    pub module_id: String,
    pub anomaly_score: f32,
    pub threat_level: ThreatLevel,
    pub tarpit_delay_us: u64,
    pub fuel_throttle_factor: f32,
    pub should_quarantine: bool,
    pub primary_indicator: String,
    pub timestamp: u64,
}

/// Online behavioral profile for a loaded module using EWMA and online variance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralProfile {
    pub module_id: String,
    pub sample_count: u64,
    pub fuel_ewma: f32,
    pub fuel_var: f32,
    pub calls_ewma: f32,
    pub denied_count: u64,
    pub last_entropy: f32,
    pub consecutive_anomalies: u32,
}

impl BehavioralProfile {
    pub fn new(module_id: impl Into<String>) -> Self {
        Self {
            module_id: module_id.into(),
            sample_count: 0,
            fuel_ewma: 100.0,
            fuel_var: 50.0,
            calls_ewma: 1.0,
            denied_count: 0,
            last_entropy: 0.5,
            consecutive_anomalies: 0,
        }
    }

    /// Update behavioral statistics with a new invocation observation.
    pub fn observe(
        &mut self,
        fuel: u64,
        host_calls: u64,
        denied: bool,
        entropy: f32,
    ) {
        let alpha = 0.20_f32; // EWMA smoothing factor
        let fuel_f = fuel as f32;
        let calls_f = host_calls as f32;

        if self.sample_count == 0 {
            self.fuel_ewma = fuel_f;
            self.fuel_var = 100.0;
            self.calls_ewma = calls_f;
        } else {
            let diff = fuel_f - self.fuel_ewma;
            self.fuel_ewma += alpha * diff;
            self.fuel_var = (1.0 - alpha) * (self.fuel_var + alpha * diff * diff);
            self.calls_ewma += alpha * (calls_f - self.calls_ewma);
        }

        if denied {
            self.denied_count = self.denied_count.saturating_add(1);
        }

        self.last_entropy = entropy;
        self.sample_count = self.sample_count.saturating_add(1);
    }
}

/// Automated fingerprint of an identified attack vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatSignature {
    pub signature_id: String,
    pub pattern_hash: String,
    pub attack_type: String,
    pub created_at: u64,
    pub severity: u32,
}

/// Central database storing learned threat signatures.
#[derive(Debug, Default)]
pub struct ThreatSignatureDatabase {
    signatures: RwLock<HashMap<String, ThreatSignature>>,
}

impl ThreatSignatureDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_attack(&self, module_id: &str, attack_type: &str, evidence: &[u8]) -> ThreatSignature {
        let mut hasher = Sha256::new();
        hasher.update(module_id.as_bytes());
        hasher.update(attack_type.as_bytes());
        hasher.update(evidence);
        let pattern_hash = hex::encode(hasher.finalize());

        let sig_id = format!("SIG-{}", &pattern_hash[..12]);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let sig = ThreatSignature {
            signature_id: sig_id.clone(),
            pattern_hash,
            attack_type: attack_type.to_string(),
            created_at: now,
            severity: 9,
        };

        self.signatures.write().insert(sig_id, sig.clone());
        sig
    }

    pub fn is_known_threat(&self, pattern_hash: &str) -> bool {
        self.signatures.read().values().any(|s| s.pattern_hash == pattern_hash)
    }

    pub fn list_signatures(&self) -> Vec<ThreatSignature> {
        self.signatures.read().values().cloned().collect()
    }
}

/// Autonomous Adaptive Threat Detector for WorldVM instances.
#[derive(Debug, Clone)]
pub struct AdaptiveThreatDetector {
    profiles: Arc<RwLock<HashMap<String, BehavioralProfile>>>,
    signatures: Arc<ThreatSignatureDatabase>,
}

impl Default for AdaptiveThreatDetector {
    fn default() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            signatures: Arc::new(ThreatSignatureDatabase::new()),
        }
    }
}

impl AdaptiveThreatDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates an invocation and computes an instantaneous anomaly score.
    pub fn evaluate(
        &self,
        module_id: &str,
        fuel_consumed: u64,
        host_calls: u64,
        had_denial: bool,
        payload_entropy: f32,
    ) -> ThreatAssessment {
        let mut profiles = self.profiles.write();
        let profile = profiles
            .entry(module_id.to_string())
            .or_insert_with(|| BehavioralProfile::new(module_id));

        // 1. Calculate Fuel Z-Score component
        let std_dev = profile.fuel_var.sqrt().max(10.0);
        let fuel_delta = (fuel_consumed as f32 - profile.fuel_ewma).max(0.0);
        let fuel_z_score = fuel_delta / std_dev;
        let fuel_component = (fuel_z_score / 6.0).clamp(0.0, 1.0) * 0.40;

        // 2. Calculate Host Call Burst component
        let call_burst = (host_calls as f32 - profile.calls_ewma).max(0.0);
        let call_component = (call_burst / 20.0).clamp(0.0, 1.0) * 0.25;

        // 3. Denied capability probe penalty (aggressive)
        let denial_component = if had_denial { 0.45 } else { 0.0 };

        // 4. Entropy component (detecting obfuscation/shellcode patterns)
        let entropy_component = if payload_entropy > 0.85 { 0.20 } else { 0.0 };

        // Combined Anomaly Score (0.0 to 1.0)
        let total_score = (fuel_component + call_component + denial_component + entropy_component).clamp(0.0, 1.0);

        // Determine Threat Level & Actions
        let (threat_level, tarpit_delay_us, fuel_throttle, should_quarantine, indicator) = if total_score >= 0.70 {
            profile.consecutive_anomalies += 1;
            let ind = if had_denial {
                "Denied Capability Probe + Zero-Day Pattern"
            } else if fuel_z_score > 4.0 {
                "Instruction Fuel Explosion (Infinite Loop Vector)"
            } else {
                "Composite Hostile Anomaly"
            };

            // Fingerprint attack
            self.signatures.record_attack(
                module_id,
                ind,
                &fuel_consumed.to_le_bytes(),
            );

            (ThreatLevel::Critical, 5_000, 0.20, true, ind.to_string())
        } else if total_score >= 0.30 {
            profile.consecutive_anomalies += 1;
            (
                ThreatLevel::Elevated,
                500,  // 500us synthetic backpressure delay
                0.50, // 50% fuel quota clamping
                false,
                "Elevated Variance: Tarpit Throttling Engaged".to_string(),
            )
        } else {
            profile.consecutive_anomalies = 0;
            (ThreatLevel::Normal, 0, 1.0, false, "Nominal Baseline".to_string())
        };

        // Update profile
        profile.observe(fuel_consumed, host_calls, had_denial, payload_entropy);

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        ThreatAssessment {
            module_id: module_id.to_string(),
            anomaly_score: total_score,
            threat_level,
            tarpit_delay_us,
            fuel_throttle_factor: fuel_throttle,
            should_quarantine,
            primary_indicator: indicator,
            timestamp: now,
        }
    }

    /// Calculate Shannon entropy of a byte payload (normalized 0.0 to 1.0).
    pub fn calculate_entropy(payload: &[u8]) -> f32 {
        if payload.is_empty() {
            return 0.0;
        }

        let mut frequencies = [0usize; 256];
        for &b in payload {
            frequencies[b as usize] += 1;
        }

        let len = payload.len() as f32;
        let mut entropy = 0.0_f32;

        for &count in &frequencies {
            if count > 0 {
                let p = count as f32 / len;
                entropy -= p * p.log2();
            }
        }

        // Max Shannon entropy for 256 symbols is log2(256) = 8.0 bits
        (entropy / 8.0).clamp(0.0, 1.0)
    }

    /// Gets current behavioral profile snapshot for a module.
    pub fn get_profile(&self, module_id: &str) -> Option<BehavioralProfile> {
        self.profiles.read().get(module_id).cloned()
    }

    /// Access the threat signature database.
    pub fn signatures(&self) -> &ThreatSignatureDatabase {
        &self.signatures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nominal_behavior_assessment() {
        let sentinel = AdaptiveThreatDetector::new();
        // Warm up baseline
        for _ in 0..10 {
            let _ = sentinel.evaluate("test-mod", 500, 2, false, 0.4);
        }

        let assessment = sentinel.evaluate("test-mod", 520, 2, false, 0.4);
        assert_eq!(assessment.threat_level, ThreatLevel::Normal);
        assert!(assessment.anomaly_score < 0.30);
        assert_eq!(assessment.tarpit_delay_us, 0);
        assert!(!assessment.should_quarantine);
    }

    #[test]
    fn test_anomaly_detection_and_tarpit() {
        let sentinel = AdaptiveThreatDetector::new();
        // Establish baseline of 500 fuel
        for _ in 0..10 {
            let _ = sentinel.evaluate("mod-a", 500, 2, false, 0.3);
        }

        // Sudden massive fuel spike (10x) + host call storm
        let assessment = sentinel.evaluate("mod-a", 50_000, 25, false, 0.4);
        assert_ne!(assessment.threat_level, ThreatLevel::Normal);
        assert!(assessment.anomaly_score >= 0.30);
        assert!(assessment.tarpit_delay_us > 0);
    }

    #[test]
    fn test_unauthorized_probe_critical_escalation() {
        let sentinel = AdaptiveThreatDetector::new();
        // Mod immediately attempts denied capability probe with high entropy payload
        let assessment = sentinel.evaluate("hostile-mod", 25_000, 5, true, 0.92);
        assert_eq!(assessment.threat_level, ThreatLevel::Critical);
        assert!(assessment.anomaly_score >= 0.70);
        assert!(assessment.should_quarantine);

        // Verify signature generated
        assert!(!sentinel.signatures().list_signatures().is_empty());
    }

    #[test]
    fn test_entropy_calculation() {
        let zero_entropy = vec![0u8; 100];
        assert_eq!(AdaptiveThreatDetector::calculate_entropy(&zero_entropy), 0.0);

        // Uniform random-like byte sequence
        let high_entropy: Vec<u8> = (0..=255).collect();
        let ent = AdaptiveThreatDetector::calculate_entropy(&high_entropy);
        assert!(ent > 0.95);
    }
}
