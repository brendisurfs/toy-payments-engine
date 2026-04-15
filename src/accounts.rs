use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serde::Serialize;

use crate::transactions::{Transaction, TransactionStatus};

/// Defines our structure for a single client.
#[derive(Debug, Serialize, Default)]
pub struct ClientAccount {
    available_funds: f32,
    total_funds: f32,
    held_funds: f32,
    client_id: u16,
    frozen: bool,
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

    pub fn freeze(&mut self) {
        self.frozen = true;
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

    #[tracing::instrument(skip(self))]
    pub fn dispute_transaction(&mut self, reference_txn: u32) {
        let Some(txn) = self.transaction_log.get_mut(&reference_txn) else {
            tracing::warn!("No found transaction");
            return;
        };

        // Only operate on Deposit transactions
        let Transaction::Deposit {
            client_id,
            amount,
            status,
            ..
        } = *txn.borrow()
        else {
            tracing::warn!("Referenced transaction is not a Deposit");
            return;
        };

        if status != TransactionStatus::Clean {
            tracing::warn!("Transaction is not clean: {status:?}");
            return;
        }

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

    /// Releases associated held funds to a disputed transaction.
    /// Funds that were previously disputed are no longer disputed.
    #[tracing::instrument(skip(self))]
    pub fn resolve_transaction(&mut self, reference_txn: u32) {
        let Some(txn) = self.transaction_log.get_mut(&reference_txn) else {
            tracing::warn!("No found transaction");
            return;
        };

        // Only operate on Deposit transactions
        let Transaction::Deposit {
            client_id,
            amount,
            status,
            ..
        } = *txn.borrow()
        else {
            tracing::warn!("Referenced transaction is not a Deposit");
            return;
        };

        if status != TransactionStatus::Disputed {
            tracing::warn!("Incorrect transaction status: {status:?}");
            return;
        }

        let Some(account) = self.accounts.get_mut(&client_id) else {
            tracing::warn!("No account exists");
            return;
        };

        tracing::trace!("Updating account funds from held funds");
        account.held_funds -= amount;
        account.available_funds += amount;
        tracing::debug!("{account:#?}");

        tracing::trace!("Updating transaction status");
        txn.borrow_mut().update_status(TransactionStatus::Resolved);
        tracing::debug!("{txn:#?}");
    }

    /// Reverses a transaction, where funds that were previously held have now been withdrawn.
    /// decreases clients held funds and total funds by the amount previously disputed.
    /// This also freezes a clients account.
    #[tracing::instrument(skip(self))]
    pub fn handle_chargeback(&mut self, reference_txn: u32) {
        let Some(txn) = self.transaction_log.get_mut(&reference_txn) else {
            tracing::warn!("No found transaction");
            return;
        };

        // Only operate on Deposit transactions
        let Transaction::Deposit {
            client_id,
            amount,
            status,
            ..
        } = *txn.borrow()
        else {
            tracing::warn!("Referenced transaction is not a Deposit");
            return;
        };

        if status != TransactionStatus::Disputed {
            tracing::warn!("Transaction is not disputed");
            return;
        }

        // If we pass our initial checks, the account must be frozen if a withdrawal occurs.
        let Some(account) = self.accounts.get_mut(&client_id) else {
            tracing::warn!("No account exists");
            return;
        };

        tracing::trace!("Freezing account");
        account.freeze();

        tracing::trace!("Updating account funds");
        account.held_funds -= amount;
        account.total_funds -= amount;
        account.available_funds -= amount;
        tracing::debug!("{account:#?}");

        txn.borrow_mut()
            .update_status(TransactionStatus::ChargedBack);
        tracing::debug!("{txn:#?}");
    }

    pub fn get_account(&mut self, client_id: u16) -> &ClientAccount {
        self.accounts.entry(client_id).or_insert(ClientAccount {
            available_funds: 0.0,
            total_funds: 0.0,
            held_funds: 0.0,
            client_id,
            frozen: false,
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
            frozen: false,
        });

        if account.frozen {
            tracing::error!("Account is frozen, unable to deposit");
            return false;
        }

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

        if account.frozen {
            tracing::error!("Account is frozen");
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

    pub fn print_accounts(&self) {
        let accts = &self.accounts;
        tracing::debug!("{accts:#?}");
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
