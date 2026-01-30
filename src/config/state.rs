use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct State {
    pub counter: Counter,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Counter {
    pub last_number: u32,
    pub last_year: u32,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            last_number: 0,
            last_year: chrono::Utc::now().year() as u32,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HistoryEntry {
    pub number: String,
    pub client: String,
    pub date: NaiveDate,
    pub total: f64,
    pub file: String,
    /// Original item inputs (e.g., ["consulting:8", "development:40"])
    #[serde(default)]
    pub items: Vec<String>,
}

use chrono::Datelike;
