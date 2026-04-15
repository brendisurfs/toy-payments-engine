use std::{collections::HashMap, ops::Sub, rc::Rc};

use anyhow::anyhow;
use serde::Serialize;

use crate::transactions::Transaction;

/// Defines our structure for a single client.
#[derive(Debug, Serialize, Default)]
pub struct ClientAccount {
    available_funds: f32,
    total_funds: f32,
    held_funds: f32,
    client_id: u16,
    locked: bool,
}
impl ClientAccount {
    pub fn new(client_id: u16) -> Self {
        Self {
            client_id,
            ..Default::default()
        }
    }
    /// The total funds available or held.
    pub fn total(&self) -> f32 {
        self.total_funds + self.held_funds
    }
}

/// primitive structure holding clients,
/// with methods to interact between client accounts.
#[derive(Default)]
pub struct AccountManager {
    /// store of our accounts to interact with.
    accounts: HashMap<u16, ClientAccount>,

    /// keep an log-like structure for our system to reference previous transactions.
    /// Note that this is a simple implementation, and should not be used in production due to excess memory allocation.
    /// Rather, a production setup would use an append-only database to reference previous transactions.
    txn_log: HashMap<u32, Rc<Transaction>>,
}

impl AccountManager {
    pub fn write_to_log(&mut self, transaction: Rc<Transaction>) {
        tracing::trace!("Write to tx_log");
        let tx_id = transaction.id();
        self.txn_log.insert(*tx_id, transaction);
    }

    /// retrieves a read-only borrow of a client account, if it exists.
    pub fn get_or_add_account(&mut self, client_id: u16) -> &ClientAccount {
        self.accounts.entry(client_id).or_insert(ClientAccount {
            available_funds: 0.0,
            total_funds: 0.0,
            held_funds: 0.0,
            client_id,
            locked: false,
        })
    }
    /// Deposits a provided amount into the account associated with the provided client_id.
    /// If the client id does not exist, we create a new account.
    pub fn deposit_to_account(&mut self, client_id: u16, amount: f32) -> bool {
        let account = self.accounts.entry(client_id).or_insert(ClientAccount {
            available_funds: 0.0,
            total_funds: 0.0,
            held_funds: 0.0,
            client_id,
            locked: false,
        });

        account.available_funds += amount;
        account.total_funds += amount;

        true
    }

    /// Withdraws an amount from an account.
    /// This will fail and return false if the account is locked
    /// or the account has insuffient funds.
    pub fn withdraw_from_account(&mut self, client_id: u16, amount: f32) -> anyhow::Result<()> {
        let Some(account) = self.accounts.get_mut(&client_id) else {
            return Err(anyhow!("Account with id {client_id} does not exist"));
        };

        if account.locked {
            return Err(anyhow!("Current account is locked"));
        }

        if account.available_funds.sub(amount) < 0.0 {
            return Err(anyhow!("Account does not have enough available funds"));
        }

        account.available_funds -= amount;
        account.total_funds -= amount;

        Ok(())
    }

    /// locks an account to prevent the account from being able to transact with other accounts.
    /// This should occur when a dispute is made.
    pub fn lock_account(&mut self, client_id: u16) {
        if let Some(account) = self.accounts.get_mut(&client_id) {
            account.locked = true;
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::accounts::AccountManager;

    #[test]
    fn test_account_withdraw() {
        let mut act_mgr = AccountManager::default();
        act_mgr.deposit_to_account(1, 10.0);
        let did_withdraw = act_mgr.withdraw_from_account(1, 11.0);
        assert!(did_withdraw.is_ok());
    }

    #[test]
    fn test_account_deposit() {
        let mut act_mgr = AccountManager::default();
        let _ = act_mgr.get_or_add_account(2);
        let did_deposit = act_mgr.deposit_to_account(2, 1.0);
        assert!(did_deposit);
    }
}
