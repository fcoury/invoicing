mod config;
mod error;
mod invoice;
mod pdf;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::config::{
    config_dir, load_clients, load_config, load_items, load_state,
    CONFIG_TEMPLATE, CLIENTS_TEMPLATE, ITEMS_TEMPLATE,
};
use crate::error::{InvoiceError, Result};
use crate::invoice::generate_invoice;

#[derive(Parser)]
#[command(name = "invoice")]
#[command(version, about = "Minimal CLI invoicing system", long_about = None)]
struct Cli {
    /// Path to config directory (default: ~/.invoice or XDG config)
    #[arg(short = 'C', long, global = true)]
    config_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize config directory with template files
    Init,

    /// Generate a new invoice
    Generate {
        /// Client identifier from clients.toml
        #[arg(short, long)]
        client: String,

        /// Line items in format "item:quantity" (can be repeated)
        #[arg(short, long, value_name = "ITEM:QTY")]
        item: Vec<String>,

        /// Custom output file path (default: output_dir/INV-XXXX.pdf)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List configured clients
    Clients,

    /// List available line items
    Items,

    /// Show invoice status and next number
    Status,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Determine config directory
    let cfg_dir = match cli.config_dir {
        Some(p) => p,
        None => config_dir()?,
    };

    match cli.command {
        Commands::Init => cmd_init(&cfg_dir),
        Commands::Generate { client, item, output } => {
            cmd_generate(&cfg_dir, &client, &item, output)
        }
        Commands::Clients => cmd_clients(&cfg_dir),
        Commands::Items => cmd_items(&cfg_dir),
        Commands::Status => cmd_status(&cfg_dir),
    }
}

/// Initialize config directory with template files
fn cmd_init(cfg_dir: &PathBuf) -> Result<()> {
    use std::fs;

    if cfg_dir.exists() {
        return Err(InvoiceError::AlreadyInitialized(cfg_dir.clone()));
    }

    // Create directories
    fs::create_dir_all(cfg_dir)?;
    fs::create_dir_all(cfg_dir.join("output"))?;
    fs::create_dir_all(cfg_dir.join("templates"))?;

    // Write template files
    fs::write(cfg_dir.join("config.toml"), CONFIG_TEMPLATE)?;
    fs::write(cfg_dir.join("clients.toml"), CLIENTS_TEMPLATE)?;
    fs::write(cfg_dir.join("items.toml"), ITEMS_TEMPLATE)?;

    println!("Initialized invoice config at: {}", cfg_dir.display());
    println!();
    println!("Next steps:");
    println!("  1. Edit your company details:  $EDITOR {}/config.toml", cfg_dir.display());
    println!("  2. Add your clients:           $EDITOR {}/clients.toml", cfg_dir.display());
    println!("  3. Configure line items:       $EDITOR {}/items.toml", cfg_dir.display());
    println!();
    println!("Then generate your first invoice:");
    println!("  invoice generate --client <client-id> --item <item>:<quantity>");

    Ok(())
}

/// List configured clients
fn cmd_clients(cfg_dir: &PathBuf) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let clients = load_clients(cfg_dir)?;

    if clients.is_empty() {
        println!("No clients configured.");
        println!("Add clients to: {}/clients.toml", cfg_dir.display());
        return Ok(());
    }

    println!("{:<20} {:<30} {}", "ID", "NAME", "EMAIL");
    println!("{}", "-".repeat(70));

    let mut sorted: Vec<_> = clients.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);

    for (id, client) in sorted {
        println!("{:<20} {:<30} {}", id, client.name, client.email);
    }

    Ok(())
}

/// List available line items
fn cmd_items(cfg_dir: &PathBuf) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let config = load_config(cfg_dir)?;
    let items = load_items(cfg_dir)?;

    if items.is_empty() {
        println!("No items configured.");
        println!("Add items to: {}/items.toml", cfg_dir.display());
        return Ok(());
    }

    println!(
        "{:<20} {:<35} {:>10} {:>10}",
        "ID", "DESCRIPTION", "RATE", "UNIT"
    );
    println!("{}", "-".repeat(80));

    let mut sorted: Vec<_> = items.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);

    for (id, item) in sorted {
        println!(
            "{:<20} {:<35} {:>9}{} {:>10}",
            id,
            truncate(&item.description, 35),
            format!("{:.2}", item.rate),
            config.invoice.currency_symbol,
            format!("/{}", item.unit)
        );
    }

    Ok(())
}

/// Show invoice status
fn cmd_status(cfg_dir: &PathBuf) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let config = load_config(cfg_dir)?;
    let clients = load_clients(cfg_dir)?;
    let items = load_items(cfg_dir)?;
    let state = load_state(cfg_dir)?;

    // Calculate next invoice number
    let current_year = chrono::Utc::now().year() as u32;
    let next_seq = if state.counter.last_year == current_year {
        state.counter.last_number + 1
    } else {
        1 // Reset for new year
    };

    let next_number = format_invoice_number(&config.invoice.number_format, current_year, next_seq);

    println!("Invoice Status");
    println!("{}", "-".repeat(40));
    println!("Config directory: {}", cfg_dir.display());
    println!("Company:          {}", config.company.name);
    println!("Clients:          {}", clients.len());
    println!("Items:            {}", items.len());
    println!("Next invoice:     {}", next_number);

    if !state.history.is_empty() {
        println!();
        println!("Recent invoices:");
        for entry in state.history.iter().rev().take(5) {
            println!("  {} - {} - {}{:.2}",
                entry.number,
                entry.client,
                config.invoice.currency_symbol,
                entry.total
            );
        }
    }

    Ok(())
}

/// Generate a new invoice
fn cmd_generate(
    cfg_dir: &PathBuf,
    client_id: &str,
    items_input: &[String],
    output: Option<PathBuf>,
) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    if items_input.is_empty() {
        return Err(InvoiceError::NoItems);
    }

    generate_invoice(cfg_dir, client_id, items_input, output)
}

/// Format invoice number from template
fn format_invoice_number(format: &str, year: u32, seq: u32) -> String {
    format
        .replace("{year}", &year.to_string())
        .replace("{seq:04}", &format!("{:04}", seq))
        .replace("{seq:05}", &format!("{:05}", seq))
        .replace("{seq:03}", &format!("{:03}", seq))
}

/// Truncate string with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

use chrono::Datelike;
