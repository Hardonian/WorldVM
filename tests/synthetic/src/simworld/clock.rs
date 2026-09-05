//! SimWorld Deterministic Clock and Seeded PRNG.

use rand::RngCore;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickRate {
    Hz30,
    Hz60,
    Hz120,
}

impl TickRate {
    pub fn delta_seconds(&self) -> f32 {
        match self {
            Self::Hz30 => 1.0 / 30.0,
            Self::Hz60 => 1.0 / 60.0,
            Self::Hz120 => 1.0 / 120.0,
        }
    }

    pub fn hz(&self) -> u32 {
        match self {
            Self::Hz30 => 30,
            Self::Hz60 => 60,
            Self::Hz120 => 120,
        }
    }

    pub fn frame_budget_us(&self) -> u64 {
        match self {
            Self::Hz30 => 33_333,
            Self::Hz60 => 16_667,
            Self::Hz120 => 8_333,
        }
    }
}

pub struct SimClock {
    pub tick: u64,
    pub rate: TickRate,
}

impl SimClock {
    pub fn new(rate: TickRate) -> Self {
        Self { tick: 0, rate }
    }

    pub fn advance(&mut self) -> (u64, f32) {
        self.tick += 1;
        (self.tick, self.rate.delta_seconds())
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.tick as f32 * self.rate.delta_seconds()
    }
}

pub struct SimRng {
    pub seed: u64,
    rng: ChaCha8Rng,
}

impl SimRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            seed,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.rng.next_u32() as f32) / (u32::MAX as f32)
    }

    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    pub fn next_bool(&mut self) -> bool {
        self.rng.next_u32() % 2 == 0
    }
}
