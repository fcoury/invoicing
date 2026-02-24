mod config;
mod error;
mod invoice;
mod pdf;

use chrono::Datelike;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tabled::{settings::Style, Table, Tabled};

use crate::config::{
    config_dir, global_config_file, load_clients, load_config, load_global_config, load_items,
    load_state, save_state,
    state::{Payment, PaymentStatus},
    CLIENTS_TEMPLATE, CONFIG_TEMPLATE, ITEMS_TEMPLATE,
};
use crate::error::{InvoiceError, Result};
use crate::invoice::{
    generate_invoice, get_invoice_path, regenerate_invoice, ReportData, ReportInvoiceRow,
    ReportPayment,
};
use crate::pdf::generate_report_pdf;

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

        /// Open generated PDF with system default viewer
        #[arg(long)]
        open: bool,
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

        /// Open regenerated PDF with system default viewer
        #[arg(long)]
        open: bool,
    },

    /// Record a payment against an invoice
    AddPayment {
        /// Invoice number or index from 'list' (e.g., 1 or INV-2026-0001)
        invoice: String,

        /// Payment amount
        amount: f64,

        /// Payment date (default: today)
        #[arg(long)]
        date: Option<String>,
    },

    /// Remove a payment from an invoice
    RemovePayment {
        /// Invoice number or index from 'list' (e.g., 1 or INV-2026-0001)
        invoice: String,

        /// 1-based index of payment to remove (default: last)
        #[arg(long)]
        index: Option<usize>,
    },

    /// Show payment history for an invoice
    Payments {
        /// Invoice number or index from 'list' (e.g., 1 or INV-2026-0001)
        invoice: String,
    },

    /// Generate a PDF report of invoices for a client
    Report {
        /// Client identifier from clients.toml
        #[arg(short, long)]
        client: String,

        /// Filter invoices from this date (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// Filter invoices to this date (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,

        /// Filter by payment status (paid, unpaid, partial)
        #[arg(long)]
        status: Option<String>,

        /// Open generated PDF with system default viewer
        #[arg(long)]
        open: bool,
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
        Commands::Generate {
            client,
            item,
            output,
            open,
        } => cmd_generate(&cfg_dir, &client, &item, output, open),
        Commands::Clients => cmd_clients(&cfg_dir),
        Commands::Items => cmd_items(&cfg_dir),
        Commands::Status { verbose } => cmd_status(&cfg_dir, verbose),
        Commands::List { limit } => cmd_invoices(&cfg_dir, limit),
        Commands::Edit { invoice, item } => cmd_edit(&cfg_dir, &invoice, &item),
        Commands::Open { invoice } => cmd_open(&cfg_dir, &invoice),
        Commands::Regenerate { invoice, open } => cmd_regenerate(&cfg_dir, &invoice, open),
        Commands::AddPayment {
            invoice,
            amount,
            date,
        } => cmd_add_payment(&cfg_dir, &invoice, amount, date),
        Commands::RemovePayment { invoice, index } => {
            cmd_remove_payment(&cfg_dir, &invoice, index)
        }
        Commands::Payments { invoice } => cmd_payments(&cfg_dir, &invoice),
        Commands::Report {
            client,
            from,
            to,
            status,
            open,
        } => cmd_report(&cfg_dir, &client, from, to, status, open),
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
    println!(
        "  1. Edit your company details:  $EDITOR {}/config.toml",
        cfg_dir.display()
    );
    println!(
        "  2. Add your clients:           $EDITOR {}/clients.toml",
        cfg_dir.display()
    );
    println!(
        "  3. Configure line items:       $EDITOR {}/items.toml",
        cfg_dir.display()
    );
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
    #[tabled(rename = "STATUS")]
    status: String,
    #[tabled(rename = "CLIENT")]
    client: String,
}

#[derive(Tabled)]
struct PaymentRow {
    #[tabled(rename = "#")]
    index: usize,
    #[tabled(rename = "DATE")]
    date: String,
    #[tabled(rename = "AMOUNT")]
    amount: String,
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

fn add_financial_footer(table: &str, total: &str, paid: &str, outstanding: &str) -> String {
    let lines: Vec<&str> = table.lines().collect();
    if lines.len() < 4 {
        return table.to_string();
    }

    // Parse the top border to discover column widths
    let top = lines[0];
    let Some(inner) = top.strip_prefix('╭').and_then(|s| s.strip_suffix('╮')) else {
        return table.to_string();
    };

    let widths: Vec<usize> = inner.split('┬').map(|p| p.chars().count()).collect();
    if widths.len() < 6 {
        return table.to_string();
    }

    // Merge columns #, NUMBER, DATE into one label cell; keep TOTAL column; drop STATUS and CLIENT
    let left_width = widths[0] + widths[1] + widths[2] + 2; // +2 for the two ┴ replaced by spaces
    let total_width = widths[3];
    let status_width = widths[4];
    let client_width = widths[5];

    let rows = [
        ("TOTAL", total),
        ("(-) PAID", paid),
        ("(=) OUTSTANDING", outstanding),
    ];

    // Strip the original bottom border and start building
    let mut out = lines[..lines.len() - 1].join("\n");
    out.push('\n');

    // First separator: merge left 3 columns, keep TOTAL, close off STATUS+CLIENT
    out.push_str(&format!(
        "├{}┴{}┴{}┼{}┼{}┴{}╯\n",
        "─".repeat(widths[0]),
        "─".repeat(widths[1]),
        "─".repeat(widths[2]),
        "─".repeat(total_width),
        "─".repeat(status_width),
        "─".repeat(client_width),
    ));

    // Summary rows with separators between them
    for (idx, (label, value)) in rows.iter().enumerate() {
        out.push_str(&format!(
            "│ {:>left$} │ {:>total$} │\n",
            label,
            value,
            left = left_width - 2,
            total = total_width - 2
        ));
        if idx < rows.len() - 1 {
            out.push_str(&format!(
                "├{}┼{}┤\n",
                "─".repeat(left_width),
                "─".repeat(total_width)
            ));
        }
    }

    // Bottom border
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
            println!(
                "  {} - {} - {}{:.2}",
                entry.number, entry.client, config.invoice.currency_symbol, entry.total
            );
        }
    }

    Ok(())
}

/// Fetch the current USD→BRL exchange rate from the Frankfurter API.
/// Returns None on any failure (network, timeout, parse error) so the
/// caller can silently skip the BRL line.
fn fetch_usd_to_brl_rate() -> Option<f64> {
    use std::time::Duration;
    use ureq::Agent;

    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .build()
        .into();

    let body: String = agent
        .get("https://api.frankfurter.dev/v1/latest?base=USD&symbols=BRL")
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;

    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    json["rates"]["BRL"].as_f64()
}

/// List generated invoices with three-way status (UNPAID / PARTIAL / PAID)
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

    // Derive status from payment records
    let rows: Vec<InvoiceRow> = invoices
        .iter()
        .map(|(idx, entry)| InvoiceRow {
            index: idx + 1,
            number: entry.number.clone(),
            date: entry.date.to_string(),
            total: format_whole_money(entry.total, &config.invoice.currency_symbol),
            status: entry.status().to_string(),
            client: entry.client.clone(),
        })
        .collect();

    // Financial summary uses actual payment amounts
    let shown_total: f64 = invoices.iter().map(|(_, entry)| entry.total).sum();
    let shown_paid: f64 = invoices.iter().map(|(_, entry)| entry.paid_amount()).sum();
    let shown_outstanding: f64 = shown_total - shown_paid;

    let table = Table::new(rows).with(Style::rounded()).to_string();
    let total_amount = format_whole_money(shown_total, &config.invoice.currency_symbol);
    let paid_amount = format_whole_money(shown_paid, &config.invoice.currency_symbol);
    let outstanding_amount = format_whole_money(shown_outstanding, &config.invoice.currency_symbol);
    let table = add_financial_footer(&table, &total_amount, &paid_amount, &outstanding_amount);

    println!("{table}");

    println!();
    println!("Total: {} invoices", state.history.len());

    // Show outstanding amount converted to BRL if there's an outstanding balance
    if shown_outstanding > 0.0 {
        if let Some(rate) = fetch_usd_to_brl_rate() {
            let brl_amount = (shown_outstanding * rate).round() as i64;
            println!(
                "Outstanding in BRL: R$ {} (1 USD = {:.2} BRL)",
                format_grouped_int(brl_amount),
                rate
            );
        }
    }

    println!(
        "Use index number with open/edit/regenerate/add-payment/remove-payment (e.g., 'invoice open 1')"
    );

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
    open: bool,
) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    if items_input.is_empty() {
        return Err(InvoiceError::NoItems);
    }

    let output_path = output.clone();
    generate_invoice(cfg_dir, client_id, items_input, output)?;
    if open {
        let pdf_path = if let Some(path) = output_path {
            path
        } else {
            let state = load_state(cfg_dir)?;
            let invoice_number = state
                .history
                .last()
                .map(|entry| entry.number.clone())
                .ok_or_else(|| InvoiceError::InvoiceNotFound("latest".to_string()))?;
            get_invoice_path(cfg_dir, &invoice_number)?
        };
        open_path(&pdf_path)?;
    }
    Ok(())
}

/// Format invoice number from template
fn format_invoice_number(format: &str, year: u32, seq: u32) -> String {
    format
        .replace("{year}", &year.to_string())
        .replace("{seq:04}", &format!("{:04}", seq))
        .replace("{seq:05}", &format!("{:05}", seq))
        .replace("{seq:03}", &format!("{:03}", seq))
}

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
        println!(
            "  Total:  {}{:.2}",
            config.invoice.currency_symbol, entry.total
        );
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

    open_path(&pdf_path)?;

    println!("Opened {}", pdf_path.display());
    Ok(())
}

fn open_path(pdf_path: &PathBuf) -> Result<()> {
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
    Ok(())
}

/// Regenerate an invoice PDF
fn cmd_regenerate(cfg_dir: &PathBuf, invoice_ref: &str, open: bool) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let invoice_number = resolve_invoice_number(cfg_dir, invoice_ref)?;
    let pdf_path = regenerate_invoice(cfg_dir, &invoice_number, None)?;
    if open {
        open_path(&pdf_path)?;
    }

    println!("Regenerated {}", invoice_number);
    println!("  Saved: {}", pdf_path.display());

    Ok(())
}

/// Record a payment against an invoice
fn cmd_add_payment(
    cfg_dir: &PathBuf,
    invoice_ref: &str,
    amount: f64,
    date_str: Option<String>,
) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    // Validate amount
    if amount <= 0.0 {
        return Err(InvoiceError::InvalidPaymentAmount);
    }

    let invoice_number = resolve_invoice_number(cfg_dir, invoice_ref)?;
    let mut state = load_state(cfg_dir)?;
    let config = load_config(cfg_dir)?;

    // Parse payment date (default to today)
    let date = match date_str {
        Some(s) => chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map_err(|_| InvoiceError::PdfGeneration(format!("Invalid --date value: '{s}'")))?,
        None => chrono::Local::now().date_naive(),
    };

    let entry = state
        .history
        .iter_mut()
        .find(|e| e.number == invoice_number)
        .ok_or_else(|| InvoiceError::InvoiceNotFound(invoice_number.clone()))?;

    // Guard against overpayment
    let remaining = entry.outstanding();
    if amount > remaining + 0.001 {
        return Err(InvoiceError::OverPayment {
            invoice: invoice_number,
            max: remaining,
        });
    }

    entry.payments.push(Payment { amount, date });
    let new_outstanding = entry.outstanding();
    let inv_number = entry.number.clone();

    save_state(cfg_dir, &state)?;

    // Print confirmation
    if new_outstanding <= 0.001 {
        println!(
            "Recorded {}{:.2} payment for {} (fully paid)",
            config.invoice.currency_symbol, amount, inv_number
        );
    } else {
        println!(
            "Recorded {}{:.2} payment for {} ({}{:.2} remaining)",
            config.invoice.currency_symbol,
            amount,
            inv_number,
            config.invoice.currency_symbol,
            new_outstanding
        );
    }

    Ok(())
}

/// Remove a payment from an invoice
fn cmd_remove_payment(
    cfg_dir: &PathBuf,
    invoice_ref: &str,
    index: Option<usize>,
) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let invoice_number = resolve_invoice_number(cfg_dir, invoice_ref)?;
    let mut state = load_state(cfg_dir)?;
    let config = load_config(cfg_dir)?;

    let entry = state
        .history
        .iter_mut()
        .find(|e| e.number == invoice_number)
        .ok_or_else(|| InvoiceError::InvoiceNotFound(invoice_number.clone()))?;

    if entry.payments.is_empty() {
        return Err(InvoiceError::NoPayments(invoice_number));
    }

    // Determine which payment to remove (1-based index, default to last)
    let remove_idx = match index {
        Some(i) => {
            if i == 0 || i > entry.payments.len() {
                return Err(InvoiceError::InvalidPaymentIndex {
                    invoice: invoice_number,
                    index: i,
                    count: entry.payments.len(),
                });
            }
            i - 1
        }
        None => entry.payments.len() - 1,
    };

    let removed = entry.payments.remove(remove_idx);
    let inv_number = entry.number.clone();

    save_state(cfg_dir, &state)?;

    println!(
        "Removed {}{:.2} payment from {}",
        config.invoice.currency_symbol, removed.amount, inv_number
    );

    Ok(())
}

/// Show payment history for an invoice
fn cmd_payments(cfg_dir: &PathBuf, invoice_ref: &str) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let invoice_number = resolve_invoice_number(cfg_dir, invoice_ref)?;
    let state = load_state(cfg_dir)?;
    let config = load_config(cfg_dir)?;

    let entry = state
        .history
        .iter()
        .find(|e| e.number == invoice_number)
        .ok_or_else(|| InvoiceError::InvoiceNotFound(invoice_number.clone()))?;

    println!("Payments for {}", invoice_number);

    if entry.payments.is_empty() {
        println!("  No payments recorded.");
    } else {
        let rows: Vec<PaymentRow> = entry
            .payments
            .iter()
            .enumerate()
            .map(|(idx, p)| PaymentRow {
                index: idx + 1,
                date: p.date.to_string(),
                amount: format!(
                    "{}{:.2}",
                    config.invoice.currency_symbol, p.amount
                ),
            })
            .collect();

        let table = Table::new(rows).with(Style::rounded()).to_string();
        println!("{table}");
    }

    println!(
        "Total paid: {}{:.2} / {}{:.2} (Status: {})",
        config.invoice.currency_symbol,
        entry.paid_amount(),
        config.invoice.currency_symbol,
        entry.total,
        entry.status()
    );

    Ok(())
}

/// Generate a PDF report of invoices for a client
fn cmd_report(
    cfg_dir: &PathBuf,
    client_id: &str,
    from: Option<String>,
    to: Option<String>,
    status: Option<String>,
    open: bool,
) -> Result<()> {
    if !cfg_dir.exists() {
        return Err(InvoiceError::ConfigNotFound(cfg_dir.clone()));
    }

    let config = load_config(cfg_dir)?;
    let clients = load_clients(cfg_dir)?;
    let state = load_state(cfg_dir)?;

    // Validate client exists
    let client = clients
        .get(client_id)
        .ok_or_else(|| InvoiceError::ClientNotFound(client_id.to_string()))?
        .clone();

    // Parse date filters
    let from_date = from
        .as_ref()
        .map(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| InvoiceError::PdfGeneration(format!("Invalid --from date: {s}")))
        })
        .transpose()?;
    let to_date = to
        .as_ref()
        .map(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| InvoiceError::PdfGeneration(format!("Invalid --to date: {s}")))
        })
        .transpose()?;

    // Validate status filter — now accepts "partial" too
    if let Some(ref s) = status {
        if s != "paid" && s != "unpaid" && s != "partial" {
            return Err(InvoiceError::PdfGeneration(format!(
                "Invalid --status value: '{s}'. Use 'paid', 'unpaid', or 'partial'."
            )));
        }
    }

    // Filter history entries for this client using three-way status
    let filtered: Vec<_> = state
        .history
        .iter()
        .filter(|e| e.client == client_id)
        .filter(|e| from_date.map_or(true, |d| e.date >= d))
        .filter(|e| to_date.map_or(true, |d| e.date <= d))
        .filter(|e| match status.as_deref() {
            Some("paid") => e.status() == PaymentStatus::Paid,
            Some("unpaid") => e.status() == PaymentStatus::Unpaid,
            Some("partial") => e.status() == PaymentStatus::Partial,
            _ => true,
        })
        .collect();

    if filtered.is_empty() {
        println!("No invoices found for client '{client_id}' with the given filters.");
        return Ok(());
    }

    // Build report rows with three-way status
    let rows: Vec<ReportInvoiceRow> = filtered
        .iter()
        .map(|e| ReportInvoiceRow {
            number: e.number.clone(),
            date: e.date.format("%B %d, %Y").to_string(),
            total: e.total,
            paid: e.paid_amount(),
            outstanding: e.outstanding(),
            payments: e
                .payments
                .iter()
                .map(|p| ReportPayment {
                    amount: p.amount,
                    date: p.date.format("%B %d, %Y").to_string(),
                })
                .collect(),
            status: e.status().to_string(),
        })
        .collect();

    // Financial summary uses actual payment amounts
    let total: f64 = filtered.iter().map(|e| e.total).sum();
    let paid: f64 = filtered.iter().map(|e| e.paid_amount()).sum();
    let outstanding = total - paid;

    let today = chrono::Local::now().format("%B %d, %Y").to_string();

    let report_data = ReportData {
        company: config.company.clone(),
        client,
        client_id: client_id.to_string(),
        rows,
        total,
        paid,
        outstanding,
        currency_symbol: config.invoice.currency_symbol.clone(),
        generated_date: today,
        filter_from: from.clone(),
        filter_to: to.clone(),
        filter_status: status.clone(),
    };

    // Determine output path
    let output_dir = config::resolve_output_dir(&config.pdf.output_dir, cfg_dir);
    std::fs::create_dir_all(&output_dir)?;

    let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let pdf_filename = format!("REPORT-{}-{}.pdf", client_id, today_str);
    let pdf_path = output_dir.join(&pdf_filename);

    // Generate PDF
    generate_report_pdf(&report_data, &pdf_path)?;

    // Print summary
    println!("Generated report for '{}'", client_id);
    println!("  Invoices: {}", filtered.len());
    println!(
        "  Total:    {}{}",
        config.invoice.currency_symbol,
        format_report_amount(total)
    );
    println!("  Saved:    {}", pdf_path.display());

    if open {
        open_path(&pdf_path)?;
    }

    Ok(())
}

/// Format a money amount with two decimal places and thousands separators
fn format_report_amount(value: f64) -> String {
    let rounded = format!("{:.2}", value);
    let parts: Vec<&str> = rounded.split('.').collect();
    let whole = parts[0];
    let frac = parts[1];

    // Group digits in the whole part
    let negative = whole.starts_with('-');
    let digits = if negative { &whole[1..] } else { whole };
    let grouped = format_grouped_int(digits.parse::<i64>().unwrap_or(0));

    if negative {
        format!("-{}.{}", grouped, frac)
    } else {
        format!("{}.{}", grouped, frac)
    }
}
