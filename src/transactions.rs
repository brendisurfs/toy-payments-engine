use std::{cell::RefCell, rc::Rc};

use rust_decimal::Decimal;
use serde::Deserialize;
use tracing::Span;

use crate::{accounts::AccountManager, cli, parser::PaymentRecord};

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
pub enum TransactionStatus {
    Clean,
    Disputed,
    Resolved,
    ChargedBack,
}

/// Since the csv crate doesnt seem to allow for tagged enum variants,
/// we need to implement our own row struct that can be parsed.
/// Note that these fields are requires to be in order:
/// tx_kind, client, tx, amount. This matches the exact row order.
#[derive(Debug, Deserialize)]
pub(crate) struct RawRow {
    #[serde(rename = "type")]
    pub tx_kind: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

#[derive(Debug, Deserialize, PartialEq)]
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
    /// Returns a reference to the id of this [`Transaction`].
    pub fn id(&self) -> &u32 {
        match self {
            Transaction::Deposit { transaction_id, .. } => transaction_id,
            Transaction::Withdrawal { transaction_id, .. } => transaction_id,
        }
    }

    /// Returns a reference to the client id of this [`Transaction`].
    pub fn client_id(&self) -> &u16 {
        match self {
            Transaction::Deposit { client_id, .. } => client_id,
            Transaction::Withdrawal { client_id, .. } => client_id,
        }
    }

    pub fn update_status(&mut self, new_status: TransactionStatus) {
        match self {
            Transaction::Deposit { status, .. } => {
                *status = new_status;
            }
            Transaction::Withdrawal { status, .. } => {
                *status = new_status;
            }
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

/// handles the next transaction event from our stream.
/// We match on our incoming event and calls the proper function to that event.
pub fn on_next_transaction(record: PaymentRecord, manager: &mut AccountManager) {
    match record {
        PaymentRecord::Transaction(txn) => {
            // we wrap the transaction in an Rc<RefCell<T>> so that we arent
            // cloning potentially a large enum variant.
            // This allows us to share across the tx_log without our memory usage
            // growing unruly in the case of a potentially long running system.
            // This does come with a slight performance overhead,
            // though not in any meaningful way here,
            // but if this were a production system, this would possibly be changed.
            // For now, we are focusing on correctness and cleanliness rather than raw hotpath performance.
            let txn = Rc::new(RefCell::new(txn));

            // write each Withdrawal and Deposit to a log we can reference later.
            manager.write_to_log(txn.clone());

            match *txn.borrow() {
                Transaction::Deposit {
                    transaction_id,
                    client_id,
                    amount,
                    ..
                } => {
                    Span::current().record("txn_id", transaction_id);
                    manager.deposit_to_account(client_id, amount)
                }
                Transaction::Withdrawal {
                    transaction_id,
                    client_id,
                    amount,
                    ..
                } => {
                    Span::current().record("txn_id", transaction_id);
                    manager.withdraw_from_account(client_id, transaction_id, amount)
                }
            };
        }

        PaymentRecord::MutatingEvent(event) => match event {
            PaymentEvent::Dispute {
                reference_txn_id,
                client_id,
            } => manager.dispute_transaction(reference_txn_id, client_id),

            PaymentEvent::Resolve {
                reference_txn_id,
                client_id,
            } => manager.resolve_transaction(reference_txn_id, client_id),

            PaymentEvent::Chargeback {
                reference_txn_id,
                client_id,
            } => manager.handle_chargeback(reference_txn_id, client_id),
        },
    }
}
