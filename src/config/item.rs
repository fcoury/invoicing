use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Item {
    pub description: String,
    pub rate: f64,
    pub unit: String,
}
