mod config;
mod error;
mod invoice;
mod pdf;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tabled::{Table, Tabled, settings::Style};

use crate::config::{
    config_dir, load_clients, load_config, load_items, load_state,
    load_global_config, global_config_file,
    CONFIG_TEMPLATE, CLIENTS_TEMPLATE, ITEMS_TEMPLATE,
};
use crate::error::{InvoiceError, Result};
use crate::invoice::{generate_invoice, get_invoice_path, regenerate_invoice};

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
    Status {
        /// Show global config information
        #[arg(short, long)]
        verbose: bool,
    },

    /// List generated invoices
    List {
        /// Number of invoices to show (default: all)
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Edit an existing invoice's line items
    Edit {
        /// Invoice number or index from 'list' (e.g., 1 or INV-2025-0001)
        invoice: String,

        /// New line items in format "item:quantity" (replaces existing items)
        #[arg(short, long, value_name = "ITEM:QTY")]
        item: Vec<String>,
    },

    /// Open an invoice PDF
    Open {
        /// Invoice number or index from 'list' (e.g., 1 or INV-2025-0001)
        invoice: String,
    },

    /// Regenerate an invoice PDF from stored data
    Regenerate {
        /// Invoice number or index from 'list' (e.g., 1 or INV-2025-0001)
        invoice: String,
    },
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
        Commands::Status { verbose } => cmd_status(&cfg_dir, verbose),
        Commands::List { limit } => cmd_invoices(&cfg_dir, limit),
        Commands::Edit { invoice, item } => cmd_edit(&cfg_dir, &invoice, &item),
        Commands::Open { invoice } => cmd_open(&cfg_dir, &invoice),
        Commands::Regenerate { invoice } => cmd_regenerate(&cfg_dir, &invoice),
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

// Table row structs for tabled
#[derive(Tabled)]
struct ClientRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "EMAIL")]
    email: String,
}

#[derive(Tabled)]
struct ItemRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "DESCRIPTION")]
    description: String,
    #[tabled(rename = "RATE")]
    rate: String,
    #[tabled(rename = "UNIT")]
    unit: String,
}

#[derive(Tabled)]
struct InvoiceRow {
    #[tabled(rename = "#")]
    index: usize,
    #[tabled(rename = "NUMBER")]
    number: String,
    #[tabled(rename = "DATE")]
    date: String,
    #[tabled(rename = "TOTAL")]
    total: String,
    #[tabled(rename = "CLIENT")]
    client: String,
}

fn format_whole_money(value: f64, currency_symbol: &str) -> String {
    let rounded = value.round() as i64;
    let grouped = format_grouped_int(rounded);
    format!("{}{:>6}", currency_symbol, grouped)
}

fn format_grouped_int(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);

    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }

    let mut grouped: String = out.chars().rev().collect();
    if negative {
        grouped.insert(0, '-');
    }
    grouped
}

fn add_total_footer(table: &str, total_amount: &str) -> String {
    let lines: Vec<&str> = table.lines().collect();
    if lines.len() < 4 {
        return table.to_string();
    }

    let top = lines[0];
    let Some(inner) = top.strip_prefix('╭').and_then(|s| s.strip_suffix('╮')) else {
        return table.to_string();
    };

    let widths: Vec<usize> = inner.split('┬').map(|p| p.chars().count()).collect();
    if widths.len() < 5 {
        return table.to_string();
    }

    let left_width = widths[0] + widths[1] + widths[2] + 2;
    let total_width = widths[3];
    let client_width = widths[4];

    let mut out = lines[..lines.len() - 1].join("\n");
    out.push('\n');
    out.push_str(&format!(
        "├{}┴{}┴{}┼{}┼{}╯",
        "─".repeat(widths[0]),
        "─".repeat(widths[1]),
        "─".repeat(widths[2]),
        "─".repeat(total_width),
        "─".repeat(client_width)
    ));
    out.push('\n');
    out.push_str(&format!(
        "│ {:>left$} │ {:>total$} │",
        "TOTAL",
        total_amount,
        left = left_width - 2,
        total = total_width - 2
    ));
    out.push('\n');
    out.push_str(&format!(
        "╰{}┴{}╯",
        "─".repeat(left_width),
        "─".repeat(total_width)
    ));

    out
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

    let mut sorted: Vec<_> = clients.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);

    let rows: Vec<ClientRow> = sorted
        .iter()
        .map(|(id, client)| ClientRow {
            id: id.to_string(),
            name: client.name.clone(),
            email: client.email.clone(),
        })
        .collect();

    let table = Table::new(rows).with(Style::rounded()).to_string();
    println!("{table}");

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

    let mut sorted: Vec<_> = items.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);

    let rows: Vec<ItemRow> = sorted
        .iter()
        .map(|(id, item)| ItemRow {
            id: id.to_string(),
            description: item.description.clone(),
            rate: format!("{:.2}{}", item.rate, config.invoice.currency_symbol),
            unit: format!("/{}", item.unit),
        })
        .collect();

    let table = Table::new(rows).with(Style::rounded()).to_string();
    println!("{table}");

    Ok(())
}

/// Show invoice status
fn cmd_status(cfg_dir: &PathBuf, show_global: bool) -> Result<()> {
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
    println!("{}", "-".repeat(50));

    // Show global config info if requested or if it exists
    let global_path = global_config_file();
    let global = load_global_config();
    if show_global || global.config_dir.is_some() {
        if global_path.exists() {
            println!("Global config:    {} (active)", global_path.display());
        } else {
            println!("Global config:    {} (not found)", global_path.display());
        }
    }

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

/// List generated invoices
fn cmd_invoices(cfg_dir: &PathBuf, limit: Option<usize>) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let config = load_config(cfg_dir)?;
    let state = load_state(cfg_dir)?;

    if state.history.is_empty() {
        println!("No invoices generated yet.");
        return Ok(());
    }

    let invoices: Vec<_> = state.history.iter().rev().enumerate().collect();
    let invoices = match limit {
        Some(n) => &invoices[..n.min(invoices.len())],
        None => &invoices[..],
    };

    let rows: Vec<InvoiceRow> = invoices
        .iter()
        .map(|(idx, entry)| InvoiceRow {
            index: idx + 1,
            number: entry.number.clone(),
            date: entry.date.to_string(),
            total: format_whole_money(entry.total, &config.invoice.currency_symbol),
            client: entry.client.clone(),
        })
        .collect();

    let shown_total: f64 = invoices.iter().map(|(_, entry)| entry.total).sum();

    let table = Table::new(rows).with(Style::rounded()).to_string();
    let total_amount = format_whole_money(shown_total, &config.invoice.currency_symbol);
    let table = add_total_footer(&table, &total_amount);

    println!("{table}");

    println!();
    println!("Total: {} invoices", state.history.len());
    println!("Use index number with open/edit/regenerate (e.g., 'invoice open 1')");

    Ok(())
}

/// Resolve an invoice reference to the actual invoice number.
/// Accepts either an index (1-based) from 'list' or the full invoice number.
fn resolve_invoice_number(cfg_dir: &PathBuf, reference: &str) -> Result<String> {
    let state = load_state(cfg_dir)?;

    // Try to parse as an index first
    if let Ok(idx) = reference.parse::<usize>() {
        if idx == 0 {
            return Err(InvoiceError::InvalidInvoiceIndex(reference.to_string()));
        }
        // Invoices are displayed in reverse order (newest first), 1-indexed
        let invoices: Vec<_> = state.history.iter().rev().collect();
        if idx > invoices.len() {
            return Err(InvoiceError::InvalidInvoiceIndex(reference.to_string()));
        }
        return Ok(invoices[idx - 1].number.clone());
    }

    // Otherwise, treat as invoice number - verify it exists
    if state.history.iter().any(|e| e.number == reference) {
        Ok(reference.to_string())
    } else {
        Err(InvoiceError::InvoiceNotFound(reference.to_string()))
    }
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

use chrono::Datelike;

/// Edit an existing invoice
fn cmd_edit(cfg_dir: &PathBuf, invoice_ref: &str, items: &[String]) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    if items.is_empty() {
        return Err(InvoiceError::NoItems);
    }

    let invoice_number = resolve_invoice_number(cfg_dir, invoice_ref)?;
    let config = load_config(cfg_dir)?;
    let pdf_path = regenerate_invoice(cfg_dir, &invoice_number, Some(items))?;

    println!("Updated {}", invoice_number);
    println!("  Items:  {}", items.join(", "));
    println!("  Saved:  {}", pdf_path.display());

    // Show new total
    let state = load_state(cfg_dir)?;
    if let Some(entry) = state.history.iter().find(|e| e.number == invoice_number) {
        println!("  Total:  {}{:.2}", config.invoice.currency_symbol, entry.total);
    }

    Ok(())
}

/// Open an invoice PDF
fn cmd_open(cfg_dir: &PathBuf, invoice_ref: &str) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let invoice_number = resolve_invoice_number(cfg_dir, invoice_ref)?;
    let pdf_path = get_invoice_path(cfg_dir, &invoice_number)?;

    // Open with system default viewer
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&pdf_path)
            .spawn()
            .map_err(|e| InvoiceError::Io(e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&pdf_path)
            .spawn()
            .map_err(|e| InvoiceError::Io(e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", pdf_path.to_str().unwrap_or("")])
            .spawn()
            .map_err(|e| InvoiceError::Io(e))?;
    }

    println!("Opened {}", pdf_path.display());
    Ok(())
}

/// Regenerate an invoice PDF
fn cmd_regenerate(cfg_dir: &PathBuf, invoice_ref: &str) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let invoice_number = resolve_invoice_number(cfg_dir, invoice_ref)?;
    let pdf_path = regenerate_invoice(cfg_dir, &invoice_number, None)?;

    println!("Regenerated {}", invoice_number);
    println!("  Saved: {}", pdf_path.display());

    Ok(())
}
