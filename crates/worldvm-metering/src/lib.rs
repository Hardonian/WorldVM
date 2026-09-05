//! WorldVM Metering — Creator monetization, marketplace revenue splits, and cryptographic compute receipts.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worldvm_core::WorldVmError;
use worldvm_signing::{sign_content, verify_raw_signature};

/// Standard platform revenue share policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueSharePolicy {
    /// Creator percentage points (e.g. 70 = 70%)
    pub creator_basis_points: u32,
    /// Game Studio percentage points (e.g. 20 = 20%)
    pub studio_basis_points: u32,
    /// WorldVM Platform percentage points (e.g. 10 = 10%)
    pub platform_basis_points: u32,
}

impl Default for RevenueSharePolicy {
    fn default() -> Self {
        Self {
            creator_basis_points: 7000,   // 70.0%
            studio_basis_points: 2000,    // 20.0%
            platform_basis_points: 1000,  // 10.0%
        }
    }
}

/// Calculated split breakdown in exact integer units (e.g. cents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevenueSplit {
    pub gross_amount: u64,
    pub creator_amount: u64,
    pub studio_amount: u64,
    pub platform_amount: u64,
}

impl RevenueSharePolicy {
    /// Compute exact integer revenue splits without floating-point inaccuracies.
    pub fn calculate_split(&self, gross_amount: u64) -> RevenueSplit {
        // Total basis points = 10,000 (100.00%)
        let creator = (gross_amount * self.creator_basis_points as u64) / 10_000;
        let studio = (gross_amount * self.studio_basis_points as u64) / 10_000;
        // Remaining goes to platform to guarantee conservation of funds
        let platform = gross_amount.saturating_sub(creator).saturating_sub(studio);

        RevenueSplit {
            gross_amount,
            creator_amount: creator,
            studio_amount: studio,
            platform_amount: platform,
        }
    }
}

/// Marketplace in-game purchase or creator tip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceTransaction {
    pub transaction_id: String,
    pub buyer_id: String,
    pub creator_id: String,
    pub game_id: String,
    pub module_id: String,
    pub item_id: String,
    pub gross_cents: u64,
    pub split: RevenueSplit,
    pub timestamp: u64,
}

/// Cryptographic compute execution receipt for hosted/dedicated server metering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeReceipt {
    pub receipt_id: String,
    pub game_id: String,
    pub module_id: String,
    pub module_hash: String,
    pub fuel_consumed: u64,
    pub memory_peak_bytes: usize,
    pub execution_time_us: u64,
    pub credits_billed: u64,
    pub content_hash: String,
    pub timestamp: u64,
    pub host_signature: Option<String>,
}

impl ComputeReceipt {
    /// Generates a canonical SHA-256 hash representing this execution record.
    pub fn compute_canonical_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.receipt_id.as_bytes());
        hasher.update(self.game_id.as_bytes());
        hasher.update(self.module_id.as_bytes());
        hasher.update(self.module_hash.as_bytes());
        hasher.update(&self.fuel_consumed.to_le_bytes());
        hasher.update(&self.execution_time_us.to_le_bytes());
        hasher.update(&self.credits_billed.to_le_bytes());
        hex::encode(hasher.finalize())
    }

    /// Sign the receipt with the host server's Ed25519 key.
    pub fn sign(&mut self, key: &ed25519_dalek::SigningKey) {
        let hash = self.compute_canonical_hash();
        self.content_hash = hash.clone();
        let sig = sign_content(key, &hash);
        self.host_signature = Some(sig.signature);
    }

    /// Verifies the cryptographic integrity of the receipt.
    pub fn verify(&self, public_key_hex: &str) -> Result<bool, WorldVmError> {
        let hash = self.compute_canonical_hash();
        if hash != self.content_hash {
            return Ok(false);
        }

        if let Some(ref sig_hex) = self.host_signature {
            verify_raw_signature(public_key_hex, &hash, sig_hex)
        } else {
            Ok(false)
        }
    }
}

/// High-throughput in-memory ledger tracking platform revenue and creator balances.
#[derive(Debug, Default)]
pub struct MarketplaceLedger {
    transactions: RwLock<Vec<MarketplaceTransaction>>,
    creator_balances: RwLock<HashMap<String, u64>>,
    platform_total_revenue_cents: RwLock<u64>,
}

impl MarketplaceLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an in-game transaction with a revenue policy.
    pub fn process_purchase(
        &self,
        buyer_id: impl Into<String>,
        creator_id: impl Into<String>,
        game_id: impl Into<String>,
        module_id: impl Into<String>,
        item_id: impl Into<String>,
        gross_cents: u64,
        policy: &RevenueSharePolicy,
    ) -> MarketplaceTransaction {
        let creator_str = creator_id.into();
        let split = policy.calculate_split(gross_cents);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let tx_id = format!("tx_{}_{}", now, gross_cents);

        let tx = MarketplaceTransaction {
            transaction_id: tx_id,
            buyer_id: buyer_id.into(),
            creator_id: creator_str.clone(),
            game_id: game_id.into(),
            module_id: module_id.into(),
            item_id: item_id.into(),
            gross_cents,
            split: split.clone(),
            timestamp: now,
        };

        // Update creator ledger
        let mut balances = self.creator_balances.write();
        let bal = balances.entry(creator_str).or_insert(0);
        *bal = bal.saturating_add(split.creator_amount);

        // Update platform revenue
        let mut platform_rev = self.platform_total_revenue_cents.write();
        *platform_rev = platform_rev.saturating_add(split.platform_amount);

        self.transactions.write().push(tx.clone());
        tx
    }

    pub fn get_creator_balance(&self, creator_id: &str) -> u64 {
        self.creator_balances.read().get(creator_id).copied().unwrap_or(0)
    }

    pub fn get_platform_revenue(&self) -> u64 {
        *self.platform_total_revenue_cents.read()
    }

    pub fn list_transactions(&self) -> Vec<MarketplaceTransaction> {
        self.transactions.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldvm_signing::generate_keypair;

    #[test]
    fn test_exact_integer_revenue_splits() {
        let policy = RevenueSharePolicy::default(); // 70 / 20 / 10
        let split = policy.calculate_split(1000); // $10.00 (1000 cents)

        assert_eq!(split.gross_amount, 1000);
        assert_eq!(split.creator_amount, 700);  // $7.00
        assert_eq!(split.studio_amount, 200);   // $2.00
        assert_eq!(split.platform_amount, 100); // $1.00

        // Invariant: sum of parts equals gross amount
        assert_eq!(
            split.creator_amount + split.studio_amount + split.platform_amount,
            split.gross_amount
        );
    }

    #[test]
    fn test_odd_amount_conservation_of_funds() {
        let policy = RevenueSharePolicy::default();
        let split = policy.calculate_split(999); // $9.99

        assert_eq!(split.creator_amount, 699);
        assert_eq!(split.studio_amount, 199);
        assert_eq!(split.platform_amount, 101); // Takes remainder

        assert_eq!(
            split.creator_amount + split.studio_amount + split.platform_amount,
            999
        );
    }

    #[test]
    fn test_marketplace_ledger_processing() {
        let ledger = MarketplaceLedger::new();
        let policy = RevenueSharePolicy::default();

        let tx = ledger.process_purchase(
            "player_99",
            "creator_neon",
            "neon-arena",
            "zombie-spawner",
            "skin_cyber_boss",
            1500, // $15.00
            &policy,
        );

        assert_eq!(tx.split.creator_amount, 1050); // $10.50
        assert_eq!(tx.split.platform_amount, 150); // $1.50

        assert_eq!(ledger.get_creator_balance("creator_neon"), 1050);
        assert_eq!(ledger.get_platform_revenue(), 150);
    }

    #[test]
    fn test_compute_receipt_signing_and_verification() {
        let (sk, pk) = generate_keypair();
        let pk_hex = hex::encode(pk.as_bytes());

        let mut receipt = ComputeReceipt {
            receipt_id: "rec_1001".to_string(),
            game_id: "neon-arena".to_string(),
            module_id: "low-gravity".to_string(),
            module_hash: "abcd1234".to_string(),
            fuel_consumed: 42_000,
            memory_peak_bytes: 4 * 1024 * 1024,
            execution_time_us: 18,
            credits_billed: 5,
            content_hash: String::new(),
            timestamp: 1700000000,
            host_signature: None,
        };

        receipt.sign(&sk);
        assert!(receipt.host_signature.is_some());

        // Verify with matching public key
        let valid = receipt.verify(&pk_hex).expect("verification should succeed");
        assert!(valid);

        // Tamper with fuel consumed
        let mut tampered = receipt.clone();
        tampered.fuel_consumed = 999_999;
        let tampered_valid = tampered.verify(&pk_hex).expect("should verify");
        assert!(!tampered_valid);
    }
}
