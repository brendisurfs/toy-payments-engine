use std::{collections::HashMap, rc::Rc};

use serde::Serialize;

use crate::transactions::Transaction;

/// Defines our structure for a single client.
#[derive(Debug, Serialize)]
pub struct ClientAccount {
    frozen: bool,
    client_id: u16,
    total_funds: f32,
    available_funds: f32,
}

/// primitive structure holding clients,
/// with methods to interact between client accounts.
pub struct AccountManager {
    /// store of our accounts to interact with.
    accounts: HashMap<u16, ClientAccount>,

    /// keep an log-like structure for our system to reference previous transactions.
    /// Note that this is a simple implementation, and should not be used in production due to excess memory allocation.
    /// Rather, a production setup would use an append-only database to reference previous transactions.
    transaction_log: HashMap<u16, Rc<Transaction>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transaction_log: HashMap::new(),
        }
    }

    pub fn write_to_log(&mut self, transaction: Rc<Transaction>) {
        tracing::trace!("Write to tx_log");
        let tx_id = transaction.transaction_id();
        self.transaction_log.insert(*tx_id, transaction);
    }

    /// retrieves a read-only borrow of a client account, if it exists.
    pub fn get_account(&mut self, client_id: u16) -> &ClientAccount {
        self.accounts.entry(client_id).or_insert(ClientAccount {
            frozen: false,
            client_id,
            total_funds: 0.0,
            available_funds: 0.0,
        })
    }
    pub fn deposit_to_account(&mut self, client_id: u16, amount: f32) -> Option<&ClientAccount> {
        let Some(account) = self.accounts.get_mut(&client_id) else {
            return None;
        };

        account.available_funds += amount;
        account.total_funds += amount;

        Some(account)
    }

    /// freezes an account to prevent the account from being able to transact with other accounts.
    /// This should occur when a dispute is made.
    pub fn freeze_account(&mut self, client_id: u16) {
        let Some(account) = self.accounts.get_mut(&client_id) else {
            return;
        };

        account.frozen = true;
    }
}
