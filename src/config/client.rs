use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Client {
    pub name: String,
    #[serde(default)]
    pub contact: Option<String>,
    pub email: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    #[serde(default)]
    pub country: Option<String>,
}
