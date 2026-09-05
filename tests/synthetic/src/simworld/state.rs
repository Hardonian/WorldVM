//! SimWorld State — In-memory deterministic gameplay state.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimPlayer {
    pub id: String,
    pub name: String,
    pub team_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub max_health: f32,
    pub xp: u64,
    pub score: i64,
    pub inventory: HashMap<String, u32>,
    pub notifications: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimEntity {
    pub id: u64,
    pub entity_type: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub is_alive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchState {
    Lobby,
    Active,
    Overtime,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimQuest {
    pub id: String,
    pub player_id: String,
    pub stage: u32,
    pub is_completed: bool,
}

/// The entire state of a simulated match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub match_id: String,
    pub match_state: MatchState,
    pub tick: u64,
    pub gravity: f32,
    pub players: HashMap<String, SimPlayer>,
    pub entities: HashMap<u64, SimEntity>,
    pub team_scores: HashMap<u32, i64>,
    pub active_quests: HashMap<String, SimQuest>,
    pub persistent_storage: HashMap<String, String>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            match_id: "match_001".to_string(),
            match_state: MatchState::Active,
            tick: 0,
            gravity: 9.81,
            players: HashMap::new(),
            entities: HashMap::new(),
            team_scores: HashMap::new(),
            active_quests: HashMap::new(),
            persistent_storage: HashMap::new(),
        }
    }
}
