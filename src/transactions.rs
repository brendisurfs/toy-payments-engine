use std::rc::Rc;

use anyhow::{anyhow, bail};
use serde::Deserialize;

use crate::{accounts::AccountManager, parser::PaymentRecord};

enum TransactionStatus {
    Clean,
    Frozen,
    Disputed,
    Resolved,
}

/// Since the csv crate doesnt seem to allow for tagged enum variants,
/// we need to implement our own row struct that can be parsed.
#[derive(Debug, Deserialize)]
pub(crate) struct RawRow {
    #[serde(rename = "type")]
    pub tx_kind: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f32>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(try_from = "RawRow", rename_all = "lowercase")]
pub enum Transaction {
    Deposit {
        #[serde(rename = "client")]
        client_id: u16,
        tx: u32,
        amount: f32,
    },
    Withdrawal {
        #[serde(rename = "client")]
        client_id: u16,
        tx: u32,
        amount: f32,
    },
}
impl Transaction {
    /// retrieves the transaction id being referenced from the Transaction.
    pub fn id(&self) -> &u32 {
        match self {
            Transaction::Deposit { tx, .. } => tx,
            Transaction::Withdrawal { tx, .. } => tx,
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
                tx: value.tx,
                client_id: value.client,
            }),

            "withdrawal" => Ok(Transaction::Withdrawal {
                amount,
                tx: value.tx,
                client_id: value.client,
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
        reference_tx: u32,
        #[serde(rename = "client")]
        client_id: u16,
    },
    Resolve {
        #[serde(rename = "tx")]
        reference_tx: u32,
        #[serde(rename = "client")]
        client_id: u16,
    },
    Chargeback {
        #[serde(rename = "tx")]
        reference_tx: u32,
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
                reference_tx: value.tx,
            }),
            "resolve" => Ok(PaymentEvent::Resolve {
                client_id: value.client,
                reference_tx: value.tx,
            }),
            "chargeback" => Ok(PaymentEvent::Chargeback {
                client_id: value.client,
                reference_tx: value.tx,
            }),
            other => Err(format!("Unknown transaction type: {other:?}")),
        }
    }
}

/// handles the next transaction event from our stream.
/// We match on our incoming event and calls the proper function to that event.
pub fn on_next_transaction(
    record: PaymentRecord,
    manager: &mut AccountManager,
) -> anyhow::Result<()> {
    tracing::trace!("Handling transaction");
    match record {
        PaymentRecord::Transaction(txn) => {
            // we wrap the transaction in an Rc so that we arent
            // cloning potentially a large enum variant.
            // This allows us to share across the tx_log without our memory usage
            // growing unruly in the case of a potentially long running system.
            // This does come with a slight performance overhead, though not in any meaningful way
            // here, but if this were a production system, this would possibly be changed.
            // For now, we are focusing on resource usage rather than raw hotpath performance.
            let txn = Rc::new(txn);

            // write each Withdrawal and Deposit to a log we can reference later.
            manager.write_to_log(txn.clone());

            return match *txn {
                Transaction::Deposit {
                    client_id, amount, ..
                } => on_deposit(manager, client_id, amount),
                Transaction::Withdrawal {
                    client_id, amount, ..
                } => on_withdrawal(manager, client_id, amount),
            };
        }

        PaymentRecord::MutatingEvent(event) => match event {
            PaymentEvent::Dispute {
                client_id,
                reference_tx,
            } => on_dispute(manager, client_id, reference_tx)?,
            PaymentEvent::Resolve {
                client_id,
                reference_tx,
            } => on_resolve(manager, client_id, reference_tx)?,
            PaymentEvent::Chargeback {
                client_id,
                reference_tx,
            } => on_chargeback(manager, client_id, reference_tx)?,
        },
    }

    Ok(())
}

/// Handles a deposit event by adding to an account with the provided client_id.
fn on_deposit(manager: &mut AccountManager, client_id: u16, amount: f32) -> anyhow::Result<()> {
    let before_account = manager.get_or_add_account(client_id);
    tracing::debug!("Before account: {before_account:?}");

    let account = manager.deposit_to_account(client_id, amount);
    tracing::debug!("After deposit: {account:?}");

    Ok(())
}

fn on_withdrawal(manager: &mut AccountManager, client_id: u16, amount: f32) -> anyhow::Result<()> {
    if !manager.withdraw_from_account(client_id, amount) {
        bail!("Unable to withdraw from account")
    }

    Ok(())
}

fn on_dispute(
    manager: &mut AccountManager,
    client_id: u16,
    reference_txn: u32,
) -> anyhow::Result<()> {
    todo!("Implement on_dispute")
}

fn on_resolve(
    manager: &mut AccountManager,
    client_id: u16,
    reference_txn: u32,
) -> anyhow::Result<()> {
    todo!("Implement on_resolve")
}

fn on_chargeback(
    manager: &mut AccountManager,
    client_id: u16,
    reference_txn: u32,
) -> anyhow::Result<()> {
    todo!("Implement on_chargeback")
}
