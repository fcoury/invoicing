mod company;
mod client;
mod item;
mod state;

pub use company::{Company, Config};
pub use client::Client;
pub use item::Item;
pub use state::{State, HistoryEntry};

use crate::error::{InvoiceError, Result};
use directories::ProjectDirs;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Get the config directory path (~/.invoice/)
pub fn config_dir() -> Result<PathBuf> {
    // First try XDG-style directories
    if let Some(proj_dirs) = ProjectDirs::from("", "", "invoice") {
        return Ok(proj_dirs.config_dir().to_path_buf());
    }

    // Fallback to ~/.invoice/
    let home = dirs_home().ok_or_else(|| {
        InvoiceError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ))
    })?;

    Ok(home.join(".invoice"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Expand ~ in paths
pub fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs_home() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

/// Load the main config.toml
pub fn load_config(config_dir: &PathBuf) -> Result<Config> {
    let path = config_dir.join("config.toml");
    if !path.exists() {
        return Err(InvoiceError::ConfigFileNotFound(path));
    }
    let content = fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|e| InvoiceError::ConfigParse { path, source: e })
}

/// Load clients.toml as a HashMap
pub fn load_clients(config_dir: &PathBuf) -> Result<HashMap<String, Client>> {
    let path = config_dir.join("clients.toml");
    if !path.exists() {
        return Err(InvoiceError::ConfigFileNotFound(path));
    }
    let content = fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|e| InvoiceError::ConfigParse { path, source: e })
}

/// Load items.toml as a HashMap
pub fn load_items(config_dir: &PathBuf) -> Result<HashMap<String, Item>> {
    let path = config_dir.join("items.toml");
    if !path.exists() {
        return Err(InvoiceError::ConfigFileNotFound(path));
    }
    let content = fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|e| InvoiceError::ConfigParse { path, source: e })
}

/// Load state.toml (creates default if missing)
pub fn load_state(config_dir: &PathBuf) -> Result<State> {
    let path = config_dir.join("state.toml");
    if !path.exists() {
        return Ok(State::default());
    }
    let content = fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|e| InvoiceError::ConfigParse { path, source: e })
}

/// Save state.toml
pub fn save_state(config_dir: &PathBuf, state: &State) -> Result<()> {
    let path = config_dir.join("state.toml");
    let content = toml::to_string_pretty(state)
        .map_err(|e| InvoiceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        )))?;
    fs::write(path, content)?;
    Ok(())
}

/// Template content for config.toml
pub const CONFIG_TEMPLATE: &str = r#"[company]
name = "Your Company Name"
address = "123 Business Street"
city = "San Francisco"
state = "CA"
zip = "94102"
country = "USA"
email = "billing@yourcompany.com"
# phone = "+1-555-123-4567"    # optional
# tax_id = "12-3456789"        # optional

[invoice]
number_format = "INV-{year}-{seq:04}"  # e.g., INV-2026-0001
currency = "USD"
currency_symbol = "$"
due_days = 30
tax_rate = 0.0  # e.g., 0.0825 for 8.25%

[pdf]
output_dir = "~/.invoice/output"
"#;

/// Template content for clients.toml
pub const CLIENTS_TEMPLATE: &str = r#"# Define your clients here. The table name (e.g., [acme]) is used
# as the client identifier in the generate command.
#
# Example:
#   invoice generate --client acme --item consulting:8

[example-client]
name = "Example Client Inc."
contact = "Jane Smith"          # optional
email = "jane@example.com"
address = "456 Client Avenue"
city = "Los Angeles"
state = "CA"
zip = "90001"
# country = "USA"               # optional, defaults to company country
"#;

/// Template content for items.toml
pub const ITEMS_TEMPLATE: &str = r#"# Define your line items here. The table name (e.g., [consulting]) is used
# as the item identifier in the generate command.
#
# Example:
#   invoice generate --client acme --item consulting:8 --item development:40

[consulting]
description = "Technical Consulting"
rate = 150.00
unit = "hour"

[development]
description = "Software Development"
rate = 125.00
unit = "hour"

[project-setup]
description = "Project Setup & Configuration"
rate = 500.00
unit = "flat"   # fixed price, quantity is typically 1
"#;
