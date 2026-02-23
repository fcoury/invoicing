use std::fmt;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct State {
    pub counter: Counter,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Counter {
    pub last_number: u32,
    pub last_year: u32,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            last_number: 0,
            last_year: chrono::Utc::now().year() as u32,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Payment {
    pub amount: f64,
    pub date: NaiveDate,
}

/// Invoice status derived from payment history
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentStatus {
    Unpaid,
    Partial,
    Paid,
}

impl fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaymentStatus::Unpaid => write!(f, "UNPAID"),
            PaymentStatus::Partial => write!(f, "PARTIAL"),
            PaymentStatus::Paid => write!(f, "PAID"),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct HistoryEntry {
    pub number: String,
    pub client: String,
    pub date: NaiveDate,
    pub total: f64,
    pub file: String,
    #[serde(default)]
    pub payments: Vec<Payment>,
    /// Original item inputs (e.g., ["consulting:8", "development:40"])
    #[serde(default)]
    pub items: Vec<String>,
}

impl HistoryEntry {
    /// Sum of all recorded payments
    pub fn paid_amount(&self) -> f64 {
        self.payments.iter().map(|p| p.amount).sum()
    }

    /// Remaining balance on this invoice
    pub fn outstanding(&self) -> f64 {
        self.total - self.paid_amount()
    }

    /// Auto-derived payment status
    pub fn status(&self) -> PaymentStatus {
        let paid = self.paid_amount();
        if paid <= 0.0 {
            PaymentStatus::Unpaid
        } else if paid >= self.total {
            PaymentStatus::Paid
        } else {
            PaymentStatus::Partial
        }
    }
}

/// Custom deserialization to handle backward compatibility.
/// Old format had `paid: bool`; new format has `payments: Vec<Payment>`.
impl<'de> Deserialize<'de> for HistoryEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Intermediate struct that accepts both old and new fields
        #[derive(Deserialize)]
        struct Raw {
            number: String,
            client: String,
            date: NaiveDate,
            total: f64,
            file: String,
            #[serde(default)]
            paid: Option<bool>,
            #[serde(default)]
            payments: Vec<Payment>,
            #[serde(default)]
            items: Vec<String>,
        }

        let raw = Raw::deserialize(deserializer)?;

        // Migrate old `paid = true` to a single full payment
        let payments = if raw.payments.is_empty() {
            match raw.paid {
                Some(true) => vec![Payment {
                    amount: raw.total,
                    date: raw.date,
                }],
                _ => vec![],
            }
        } else {
            raw.payments
        };

        Ok(HistoryEntry {
            number: raw.number,
            client: raw.client,
            date: raw.date,
            total: raw.total,
            file: raw.file,
            payments,
            items: raw.items,
        })
    }
}
