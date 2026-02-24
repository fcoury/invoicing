use serde::Serialize;

use crate::config::{Client, Company};

/// A single payment line item for display in report detail rows
#[derive(Debug, Serialize)]
pub struct ReportPayment {
    pub amount: f64,
    pub date: String,
}

/// A single row in the invoice report table
#[derive(Debug, Serialize)]
pub struct ReportInvoiceRow {
    pub number: String,
    pub date: String,
    pub total: f64,
    pub paid: f64,
    pub outstanding: f64,
    pub payments: Vec<ReportPayment>,
    pub status: String,
}

/// Complete data for rendering the invoice report PDF
#[derive(Debug, Serialize)]
pub struct ReportData {
    pub company: Company,
    pub client: Client,
    pub client_id: String,
    pub rows: Vec<ReportInvoiceRow>,
    pub total: f64,
    pub paid: f64,
    pub outstanding: f64,
    pub currency_symbol: String,
    pub generated_date: String,
    pub filter_from: Option<String>,
    pub filter_to: Option<String>,
    pub filter_status: Option<String>,
}
