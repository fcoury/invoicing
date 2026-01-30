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
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Global configuration from ~/.config/invoicing.toml
#[derive(Debug, Deserialize, Default)]
pub struct GlobalConfig {
    /// Base directory containing config files (config.toml, clients.toml, etc.)
    pub config_dir: Option<String>,
}

/// Path to the global config file
fn global_config_path() -> Option<PathBuf> {
    dirs_home().map(|h| h.join(".config").join("invoicing.toml"))
}

/// Load global config from ~/.config/invoicing.toml
pub fn load_global_config() -> GlobalConfig {
    if let Some(path) = global_config_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
    }
    GlobalConfig::default()
}

/// Get the config directory path
/// Priority: 1) CLI flag (-C), 2) ~/.config/invoicing.toml, 3) XDG/default
pub fn config_dir() -> Result<PathBuf> {
    // Check global config first
    let global = load_global_config();
    if let Some(dir) = global.config_dir {
        return Ok(expand_path(&dir));
    }

    // Try XDG-style directories
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

/// Resolve output_dir relative to cfg_dir if it's a relative path
pub fn resolve_output_dir(output_dir: &str, cfg_dir: &Path) -> PathBuf {
    let expanded = expand_path(output_dir);
    if expanded.is_relative() {
        cfg_dir.join(expanded)
    } else {
        expanded
    }
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
output_dir = "./output"
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

/// Template content for global config (~/.config/invoicing.toml)
#[allow(dead_code)]
pub const GLOBAL_CONFIG_TEMPLATE: &str = r#"# Global invoice configuration
# This file controls where the invoice CLI looks for its data files.
#
# If this file doesn't exist, the CLI uses:
#   - macOS/Linux: ~/.config/invoice/ (XDG) or ~/.invoice/
#
# You can also override with the -C flag:
#   invoice -C /path/to/config status

# Base directory containing config.toml, clients.toml, items.toml, state.toml
config_dir = "~/.invoice"
"#;

/// Get the global config file path for display
pub fn global_config_file() -> PathBuf {
    global_config_path().unwrap_or_else(|| PathBuf::from("~/.config/invoicing.toml"))
}
