use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InvoiceError {
    #[error("Config directory not found at {0}. Run 'invoice init' to create it.")]
    ConfigNotFound(PathBuf),

    #[error("Config file not found: {0}")]
    ConfigFileNotFound(PathBuf),

    #[error("Failed to parse config file {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("Client '{0}' not found in clients.toml")]
    ClientNotFound(String),

    #[error("Item '{0}' not found in items.toml")]
    ItemNotFound(String),

    #[error("Invalid quantity '{qty}' for item '{item}': {reason}")]
    InvalidQuantity {
        item: String,
        qty: String,
        reason: String,
    },

    #[error("Invalid item format '{0}'. Expected 'item:quantity' (e.g., 'consulting:8')")]
    InvalidItemFormat(String),

    #[error("No items specified. Use --item <name>:<quantity> to add line items.")]
    NoItems,

    #[error("Typst not found. Install it from https://typst.app/ or run: cargo install typst-cli")]
    TypstNotFound,

    #[error("Failed to generate PDF: {0}")]
    PdfGeneration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config directory already exists at {0}")]
    AlreadyInitialized(PathBuf),

    #[error("Invoice '{0}' not found in history")]
    InvoiceNotFound(String),

    #[error("Invalid invoice index '{0}'. Use 'invoice list' to see available invoices.")]
    InvalidInvoiceIndex(String),

    #[error("Invoice '{0}' has no stored items (generated before item tracking was added)")]
    NoStoredItems(String),

    #[error("Invoice file not found: {0}")]
    InvoiceFileNotFound(PathBuf),

    #[error("Payment would exceed invoice total (max ${max:.2} remaining)")]
    OverPayment { invoice: String, max: f64 },

    #[error("No payments recorded for {0}")]
    NoPayments(String),

    #[error("Invalid payment index {index} for {invoice} (only {count} payment(s) recorded)")]
    InvalidPaymentIndex {
        invoice: String,
        index: usize,
        count: usize,
    },

    #[error("Payment amount must be greater than zero")]
    InvalidPaymentAmount,
}

pub type Result<T> = std::result::Result<T, InvoiceError>;
