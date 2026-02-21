use chrono::{Datelike, Local, NaiveDate};
use serde::Serialize;
use std::path::PathBuf;

use crate::config::{
    load_clients, load_config, load_items, load_state, resolve_output_dir, save_state, Client,
    Company, HistoryEntry,
};
use crate::error::{InvoiceError, Result};
use crate::pdf::generate_pdf;

/// A line item on the invoice
#[derive(Debug, Serialize)]
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub rate: f64,
    pub amount: f64,
}

/// Complete invoice data for PDF generation
#[derive(Debug, Serialize)]
pub struct InvoiceData {
    pub number: String,
    pub date: String,
    pub due_date: String,
    pub company: Company,
    pub client: Client,
    pub items: Vec<InvoiceLineItem>,
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub currency_symbol: String,
    pub due_days: u32,
    pub payment_terms: String,
}

/// Parse item input like "consulting:8" into (item_id, quantity)
fn parse_item_input(input: &str) -> Result<(&str, f64)> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 2 {
        return Err(InvoiceError::InvalidItemFormat(input.to_string()));
    }

    let item_id = parts[0];
    let qty_str = parts[1];

    let quantity: f64 = qty_str.parse().map_err(|_| InvoiceError::InvalidQuantity {
        item: item_id.to_string(),
        qty: qty_str.to_string(),
        reason: "must be a number".to_string(),
    })?;

    if quantity <= 0.0 {
        return Err(InvoiceError::InvalidQuantity {
            item: item_id.to_string(),
            qty: qty_str.to_string(),
            reason: "must be greater than 0".to_string(),
        });
    }

    Ok((item_id, quantity))
}

/// Format invoice number from template
fn format_invoice_number(format: &str, year: u32, seq: u32) -> String {
    format
        .replace("{year}", &year.to_string())
        .replace("{seq:04}", &format!("{:04}", seq))
        .replace("{seq:05}", &format!("{:05}", seq))
        .replace("{seq:03}", &format!("{:03}", seq))
}

/// Regenerate an existing invoice from stored data
pub fn regenerate_invoice(
    cfg_dir: &PathBuf,
    invoice_number: &str,
    new_items: Option<&[String]>,
) -> Result<PathBuf> {
    let config = load_config(cfg_dir)?;
    let clients = load_clients(cfg_dir)?;
    let items_catalog = load_items(cfg_dir)?;
    let mut state = load_state(cfg_dir)?;

    // Find the invoice in history
    let entry_idx = state
        .history
        .iter()
        .position(|e| e.number == invoice_number)
        .ok_or_else(|| InvoiceError::InvoiceNotFound(invoice_number.to_string()))?;

    let entry = &state.history[entry_idx];
    let client_id = entry.client.clone();
    let original_date = entry.date;

    // Use new items if provided, otherwise use stored items
    let items_to_use: Vec<String> = match new_items {
        Some(items) => items.to_vec(),
        None => {
            if entry.items.is_empty() {
                return Err(InvoiceError::NoStoredItems(invoice_number.to_string()));
            }
            entry.items.clone()
        }
    };

    // Look up client
    let client = clients
        .get(&client_id)
        .ok_or_else(|| InvoiceError::ClientNotFound(client_id.clone()))?
        .clone();

    // Parse and validate items
    let mut line_items: Vec<InvoiceLineItem> = Vec::new();

    for input in &items_to_use {
        let (item_id, quantity) = parse_item_input(input)?;

        let item = items_catalog
            .get(item_id)
            .ok_or_else(|| InvoiceError::ItemNotFound(item_id.to_string()))?;

        let amount = item.rate * quantity;

        line_items.push(InvoiceLineItem {
            description: item.description.clone(),
            quantity,
            unit: item.unit.clone(),
            rate: item.rate,
            amount,
        });
    }

    // Calculate totals
    let subtotal: f64 = line_items.iter().map(|i| i.amount).sum();
    let tax_amount = subtotal * config.invoice.tax_rate;
    let total = subtotal + tax_amount;

    // Use original date for display
    let invoice_date = original_date.format("%B %d, %Y").to_string();
    let due_date = original_date
        .checked_add_signed(chrono::Duration::days(config.invoice.due_days as i64))
        .unwrap_or(original_date)
        .format("%B %d, %Y")
        .to_string();

    // Build invoice data
    let invoice_data = InvoiceData {
        number: invoice_number.to_string(),
        date: invoice_date,
        due_date,
        company: config.company.clone(),
        client: client.clone(),
        items: line_items,
        subtotal,
        tax_rate: config.invoice.tax_rate * 100.0,
        tax_amount,
        total,
        currency_symbol: config.invoice.currency_symbol.clone(),
        due_days: config.invoice.due_days,
        payment_terms: format!("Net {} days", config.invoice.due_days),
    };

    // Determine output path
    let output_dir = resolve_output_dir(&config.pdf.output_dir, cfg_dir);
    std::fs::create_dir_all(&output_dir)?;

    let pdf_filename = format!("{}.pdf", invoice_number);
    let pdf_path = output_dir.join(&pdf_filename);

    // Generate PDF
    generate_pdf(&invoice_data, &pdf_path)?;

    // Update history entry if items changed
    if new_items.is_some() {
        state.history[entry_idx].items = items_to_use;
        state.history[entry_idx].total = total;
        save_state(cfg_dir, &state)?;
    }

    Ok(pdf_path)
}

/// Get the PDF path for an invoice
pub fn get_invoice_path(cfg_dir: &PathBuf, invoice_number: &str) -> Result<PathBuf> {
    let config = load_config(cfg_dir)?;
    let state = load_state(cfg_dir)?;

    let entry = state
        .history
        .iter()
        .find(|e| e.number == invoice_number)
        .ok_or_else(|| InvoiceError::InvoiceNotFound(invoice_number.to_string()))?;

    let output_dir = resolve_output_dir(&config.pdf.output_dir, cfg_dir);
    let pdf_path = output_dir.join(&entry.file);

    if !pdf_path.exists() {
        return Err(InvoiceError::InvoiceFileNotFound(pdf_path));
    }

    Ok(pdf_path)
}

/// Generate a new invoice
pub fn generate_invoice(
    cfg_dir: &PathBuf,
    client_id: &str,
    items_input: &[String],
    output_path: Option<PathBuf>,
) -> Result<()> {
    // Load all config
    let config = load_config(cfg_dir)?;
    let clients = load_clients(cfg_dir)?;
    let items_catalog = load_items(cfg_dir)?;
    let mut state = load_state(cfg_dir)?;

    // Look up client
    let client = clients
        .get(client_id)
        .ok_or_else(|| InvoiceError::ClientNotFound(client_id.to_string()))?
        .clone();

    // Parse and validate items
    let mut line_items: Vec<InvoiceLineItem> = Vec::new();

    for input in items_input {
        let (item_id, quantity) = parse_item_input(input)?;

        let item = items_catalog
            .get(item_id)
            .ok_or_else(|| InvoiceError::ItemNotFound(item_id.to_string()))?;

        let amount = item.rate * quantity;

        line_items.push(InvoiceLineItem {
            description: item.description.clone(),
            quantity,
            unit: item.unit.clone(),
            rate: item.rate,
            amount,
        });
    }

    // Calculate totals
    let subtotal: f64 = line_items.iter().map(|i| i.amount).sum();
    let tax_amount = subtotal * config.invoice.tax_rate;
    let total = subtotal + tax_amount;

    // Determine invoice number
    let today = Local::now();
    let current_year = today.year() as u32;

    let seq = if state.counter.last_year == current_year {
        state.counter.last_number + 1
    } else {
        1 // Reset for new year
    };

    let invoice_number = format_invoice_number(&config.invoice.number_format, current_year, seq);

    // Calculate dates
    let invoice_date = today.format("%B %d, %Y").to_string();
    let due_date = today
        .checked_add_signed(chrono::Duration::days(config.invoice.due_days as i64))
        .unwrap_or(today)
        .format("%B %d, %Y")
        .to_string();

    // Build invoice data
    let invoice_data = InvoiceData {
        number: invoice_number.clone(),
        date: invoice_date,
        due_date,
        company: config.company.clone(),
        client: client.clone(),
        items: line_items,
        subtotal,
        tax_rate: config.invoice.tax_rate * 100.0, // Convert to percentage
        tax_amount,
        total,
        currency_symbol: config.invoice.currency_symbol.clone(),
        due_days: config.invoice.due_days,
        payment_terms: format!("Net {} days", config.invoice.due_days),
    };

    // Determine output path
    let output_dir = resolve_output_dir(&config.pdf.output_dir, cfg_dir);
    std::fs::create_dir_all(&output_dir)?;

    let pdf_filename = format!("{}.pdf", invoice_number);
    let pdf_path = output_path.unwrap_or_else(|| output_dir.join(&pdf_filename));

    // Generate PDF
    generate_pdf(&invoice_data, &pdf_path)?;

    // Update state
    state.counter.last_number = seq;
    state.counter.last_year = current_year;
    state.history.push(HistoryEntry {
        number: invoice_number.clone(),
        client: client_id.to_string(),
        date: NaiveDate::from_ymd_opt(today.year(), today.month(), today.day()).unwrap(),
        total,
        file: pdf_filename,
        paid: false,
        items: items_input.to_vec(),
    });

    save_state(cfg_dir, &state)?;

    // Print summary
    println!("Generated {}", invoice_number);
    println!("  Client: {}", client.name);
    println!("  Total:  {}{:.2}", config.invoice.currency_symbol, total);
    println!("  Saved:  {}", pdf_path.display());

    Ok(())
}
