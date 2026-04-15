use std::{cell::RefCell, collections::HashMap, ops::DerefMut, rc::Rc};

use serde::Serialize;

use crate::transactions::{Transaction, TransactionStatus};

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
    transaction_log: HashMap<u32, Rc<RefCell<Transaction>>>,
}

impl AccountManager {
    pub fn write_to_log(&mut self, transaction: Rc<RefCell<Transaction>>) {
        tracing::trace!("Write to tx_log");
        let txn_id = *transaction.borrow().id();
        self.transaction_log.insert(txn_id, transaction);
    }

    /// Retrieves a reference to a Transaction from the transaction log, if it exists.
    pub fn get_transaction(&self, reference_txn: u32) -> Option<Rc<RefCell<Transaction>>> {
        self.transaction_log.get(&reference_txn).cloned()
    }

    #[tracing::instrument(skip(self))]
    pub fn dispute_transaction(&mut self, reference_txn: u32) {
        let Some(txn) = self.transaction_log.get_mut(&reference_txn) else {
            tracing::warn!("No found transaction");
            return;
        };

        let (client_id, amount) = {
            let borrower = txn.borrow();
            (*borrower.client_id(), *borrower.amount())
        };

        let Some(acct) = self.accounts.get_mut(&client_id) else {
            tracing::warn!("No account exists");
            return;
        };

        // 1. available funds should decrease by the amount disputed
        // 2. held funds should increase by the amount disputed
        // 3. total funds remain the same
        tracing::debug!(txn_amount = amount);
        acct.available_funds -= amount;
        acct.held_funds += amount;

        txn.borrow_mut().update_status(TransactionStatus::Disputed);
        tracing::debug!("{acct:#?}");
    }

    /// retrieves a read-only borrow of a client account, if it exists.
    /// if the account does not exist with the provided client_id,
    /// a new one is created and returned.
    pub fn get_account(&mut self, client_id: u16) -> &ClientAccount {
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
    #[tracing::instrument(skip(self, amount))]
    pub fn withdraw_from_account(&mut self, client_id: u16, amount: f32) -> bool {
        let Some(account) = self.accounts.get_mut(&client_id) else {
            tracing::error!("Account does not exist");
            return false;
        };

        if account.locked {
            tracing::error!("Account is locked");
            return false;
        }

        if account.available_funds - amount < 0.0 {
            tracing::warn!("Insufficient available funds");
            return false;
        }

        account.available_funds -= amount;
        account.total_funds -= amount;

        true
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
        assert!(did_withdraw);
    }

    #[test]
    fn test_account_deposit() {
        let mut act_mgr = AccountManager::default();
        let _ = act_mgr.get_account(2);
        let did_deposit = act_mgr.deposit_to_account(2, 1.0);
        assert!(did_deposit);
    }
}
