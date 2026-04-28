use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
pub enum TransactionStatus {
    Clean,
    Disputed,
    Resolved,
    ChargedBack,
}

// Newtype wrapper for ClientID and TransactionID,
// so that the two cant be passed to each other.
// struct ClientID(u16);

// struct TransactionId(u32);

/// Since the csv crate doesnt seem to allow for tagged enum variants,
/// we need to implement our own row struct that can be parsed.
/// Note that these fields are requires to be in order:
/// `tx_kind`, `client`, `tx`, `amount`. This matches the exact row order.
#[derive(Debug, Deserialize)]
pub(crate) struct RawRow {
    #[serde(rename = "type")]
    pub tx_kind: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(try_from = "RawRow", rename_all = "lowercase")]
pub enum Transaction {
    Deposit {
        #[serde(rename = "client")]
        client_id: u16,
        transaction_id: u32,
        amount: Decimal,
        status: TransactionStatus,
    },
    Withdrawal {
        #[serde(rename = "client")]
        client_id: u16,
        transaction_id: u32,
        amount: Decimal,
        status: TransactionStatus,
    },
}

impl Transaction {
    /// Returns a reference to the client id of this [`Transaction`].
    pub fn client_id(&self) -> u16 {
        match self {
            Transaction::Deposit { client_id, .. } | Transaction::Withdrawal { client_id, .. } => {
                *client_id
            }
        }
    }

    /// Returns a reference to the id of this [`Transaction`].
    pub fn id(&self) -> u32 {
        match self {
            Transaction::Deposit { transaction_id, .. }
            | Transaction::Withdrawal { transaction_id, .. } => *transaction_id,
        }
    }
}

impl TryFrom<RawRow> for Transaction {
    type Error = String;
    fn try_from(value: RawRow) -> Result<Self, Self::Error> {
        let Some(amount) = value.amount else {
            return Err("Amount is necessary".to_string());
        };

        match value.tx_kind.trim() {
            "deposit" => Ok(Transaction::Deposit {
                amount,
                client_id: value.client,
                transaction_id: value.tx,
                status: TransactionStatus::Clean,
            }),

            "withdrawal" => Ok(Transaction::Withdrawal {
                amount,
                client_id: value.client,
                transaction_id: value.tx,
                status: TransactionStatus::Clean,
            }),

            other => Err(format!("Invalid transaction type: {other}")),
        }
    }
}

/// The different mutating events we can have within our payments engine.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(try_from = "RawRow", rename_all = "lowercase")]
pub enum PaymentEvent {
    Dispute {
        #[serde(rename = "tx")]
        reference_txn_id: u32,
        #[serde(rename = "client")]
        client_id: u16,
    },
    Resolve {
        #[serde(rename = "tx")]
        reference_txn_id: u32,
        #[serde(rename = "client")]
        client_id: u16,
    },
    Chargeback {
        #[serde(rename = "tx")]
        reference_txn_id: u32,
        #[serde(rename = "client")]
        client_id: u16,
    },
}
impl PaymentEvent {
    pub fn txn_id(&self) -> u32 {
        match self {
            Self::Dispute {
                reference_txn_id, ..
            }
            | Self::Resolve {
                reference_txn_id, ..
            }
            | Self::Chargeback {
                reference_txn_id, ..
            } => *reference_txn_id,
        }
    }
}

impl TryFrom<RawRow> for PaymentEvent {
    type Error = String;
    fn try_from(value: RawRow) -> Result<Self, Self::Error> {
        match value.tx_kind.trim() {
            "dispute" => Ok(PaymentEvent::Dispute {
                client_id: value.client,
                reference_txn_id: value.tx,
            }),
            "resolve" => Ok(PaymentEvent::Resolve {
                client_id: value.client,
                reference_txn_id: value.tx,
            }),
            "chargeback" => Ok(PaymentEvent::Chargeback {
                client_id: value.client,
                reference_txn_id: value.tx,
            }),

            other => Err(format!("Unknown transaction type: {other:?}")),
        }
    }
}
