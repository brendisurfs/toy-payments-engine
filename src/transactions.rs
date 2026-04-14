use std::rc::Rc;

use anyhow::bail;
use serde::Deserialize;

use crate::accounts::AccountManager;

/// Since the csv crate doesnt seem to allow for tagged enum variants,
/// we need to implement our own row struct that can be parsed.
#[derive(Debug, Deserialize)]
pub(crate) struct RawRow {
    #[serde(rename = "type")]
    pub(crate) tx_kind: String,
    pub(crate) client: u16,
    pub(crate) tx: u16,
    pub(crate) amount: Option<f32>,
}

/// The different actions we can have within our payments engine.
/// Note that Dispute, Resolve, and Chargeback do not have amounts,
/// as they reference the amount from the transaction ID (tx).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(try_from = "RawRow", rename_all = "lowercase")]
pub enum Transaction {
    Deposit {
        #[serde(rename = "client")]
        client_id: u16,
        tx: u16,
        amount: f32,
    },
    Withdrawal {
        #[serde(rename = "client")]
        client_id: u16,
        tx: u16,
        amount: f32,
    },
    Dispute {
        #[serde(rename = "client")]
        client_id: u16,

        #[serde(rename = "tx")]
        reference_tx: u16,
    },
    Resolve {
        #[serde(rename = "client")]
        client_id: u16,

        #[serde(rename = "tx")]
        reference_tx: u16,
    },
    Chargeback {
        #[serde(rename = "client")]
        client_id: u16,

        #[serde(rename = "tx")]
        reference_tx: u16,
    },
}
impl Transaction {
    /// retrieves the transaction id being referenced from the Transaction.
    pub fn transaction_id(&self) -> &u16 {
        match self {
            Transaction::Deposit { tx, .. } => tx,
            Transaction::Withdrawal { tx, .. } => tx,
            Transaction::Dispute { reference_tx, .. } => reference_tx,
            Transaction::Resolve { reference_tx, .. } => reference_tx,
            Transaction::Chargeback { reference_tx, .. } => reference_tx,
        }
    }
}
impl TryFrom<RawRow> for Transaction {
    type Error = String;
    fn try_from(value: RawRow) -> Result<Self, Self::Error> {
        match value.tx_kind.trim() {
            "deposit" => {
                let amount = value.amount.ok_or("Deposit requires an amount")?;
                Ok(Transaction::Deposit {
                    client_id: value.client,
                    tx: value.tx,
                    amount,
                })
            }
            "withdrawal" => {
                let amount = value.amount.ok_or("Deposit requires an amount")?;
                Ok(Transaction::Withdrawal {
                    client_id: value.client,
                    tx: value.tx,
                    amount,
                })
            }
            "dispute" => Ok(Transaction::Dispute {
                client_id: value.client,
                reference_tx: value.tx,
            }),
            "resolve" => Ok(Transaction::Resolve {
                client_id: value.client,
                reference_tx: value.tx,
            }),
            "chargeback" => Ok(Transaction::Chargeback {
                client_id: value.client,
                reference_tx: value.tx,
            }),
            other => Err(format!("Unknown transaction type: {other:?}")),
        }
    }
}

/// handles the next transaction event from our stream.
/// We match on our incoming event and calls the proper function to that event.
pub fn on_transaction(
    transaction: Rc<Transaction>,
    manager: &mut AccountManager,
) -> anyhow::Result<()> {
    tracing::trace!("Handling transaction");
    match *transaction {
        Transaction::Deposit { .. } => on_deposit(manager, transaction)?,
        Transaction::Withdrawal { .. } => on_withdrawal(manager, transaction)?,

        Transaction::Dispute {
            reference_tx,
            client_id,
        } => on_dispute(manager, client_id, reference_tx)?,

        Transaction::Resolve {
            client_id,
            reference_tx,
        } => on_resolve(manager, client_id, reference_tx)?,

        Transaction::Chargeback {
            client_id,
            reference_tx,
        } => on_chargeback(manager, client_id, reference_tx)?,
    }

    Ok(())
}

fn on_deposit(manager: &mut AccountManager, transaction: Rc<Transaction>) -> anyhow::Result<()> {
    let Transaction::Deposit {
        tx,
        amount,
        client_id,
    } = *transaction
    else {
        bail!("Provided transaction is not Deposit");
    };

    manager.write_to_log(transaction);

    let before_account = manager.get_account(client_id);
    tracing::debug!("Before account: {before_account:?}");

    let account = manager.deposit_to_account(client_id, amount);
    tracing::debug!("After deposit: {account:?}");

    todo!("Implement on deposit")
}

fn on_withdrawal(manager: &mut AccountManager, transaction: Rc<Transaction>) -> anyhow::Result<()> {
    todo!("Implement on_withdrawal")
}

fn on_dispute(
    manager: &mut AccountManager,
    client_id: u16,
    reference_tx: u16,
) -> anyhow::Result<()> {
    todo!("Implement on_dispute")
}

fn on_resolve(
    manager: &mut AccountManager,
    client_id: u16,
    reference_tx: u16,
) -> anyhow::Result<()> {
    todo!("Implement on_resolve")
}

fn on_chargeback(
    manager: &mut AccountManager,
    client_id: u16,
    reference_tx: u16,
) -> anyhow::Result<()> {
    todo!("Implement on_chargeback")
}
