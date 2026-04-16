use std::collections::{hash_map::Entry, HashMap};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Serialize;

use crate::transactions::{Transaction, TransactionStatus};
use tracing::{debug, error, trace, warn};

/// Defines our structure for a single client.
#[derive(Debug, Default, Serialize)]
pub struct ClientAccount {
    #[serde(rename = "client")]
    client_id: u16,

    #[serde(rename = "available")]
    available_funds: Decimal,

    #[serde(rename = "held")]
    held_funds: Decimal,

    #[serde(rename = "total")]
    total_funds: Decimal,

    #[serde(rename = "locked")]
    frozen: bool,
}

impl ClientAccount {
    pub fn new(client_id: u16) -> Self {
        Self {
            client_id,
            ..Default::default()
        }
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub fn rounded(&self) -> ClientAccount {
        ClientAccount {
            client_id: self.client_id,
            available_funds: self.available_funds.round_dp(4),
            held_funds: self.held_funds.round_dp(4),
            total_funds: self.total_funds.round_dp(4),
            frozen: self.frozen,
        }
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
    transaction_log: HashMap<u32, Transaction>,
}

impl AccountManager {
    #[tracing::instrument(skip(self, transaction), fields(client_id = transaction.client_id(), txn = transaction.id()))]
    pub fn write_to_log(&mut self, transaction: Transaction) {
        trace!("TXN_LOG_WRITE");
        let txn_id = transaction.id();

        match self.transaction_log.entry(txn_id) {
            Entry::Occupied(_) => {
                warn!("Duplicate transaction id {txn_id}, ignoring");
            }
            Entry::Vacant(entry) => {
                entry.insert(transaction);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn dispute_transaction(&mut self, ref_txn_id: u32, ref_client_id: u16) {
        let Some(Transaction::Deposit {
            client_id,
            amount,
            status,
            ..
        }) = self.transaction_log.get_mut(&ref_txn_id)
        else {
            warn!("No Deposit transaction found with id {ref_txn_id}");
            return;
        };

        if client_id != &ref_client_id {
            warn!(
                client_id = client_id,
                ref_client_id = ref_client_id,
                "Mismatching client ids"
            );
            return;
        }

        if *status != TransactionStatus::Clean {
            warn!("Transaction is not clean: {status:?}");
            return;
        }

        let Some(acct) = self.accounts.get_mut(&client_id) else {
            warn!("No account exists");
            return;
        };

        trace!("UPDATE_ACCT_FUNDS");
        acct.available_funds -= *amount;
        acct.held_funds += *amount;

        trace!("TXN_STATUS_UPDATE");
        *status = TransactionStatus::Disputed;
    }

    /// Releases associated held funds to a disputed transaction.
    /// Funds that were previously disputed are no longer disputed.
    #[tracing::instrument(skip(self))]
    pub fn resolve_transaction(&mut self, ref_txn_id: u32, ref_client_id: u16) {
        let Some(Transaction::Deposit {
            client_id,
            amount,
            status,
            ..
        }) = self.transaction_log.get_mut(&ref_txn_id)
        else {
            warn!("No Deposit transaction found with id {ref_txn_id}");
            return;
        };

        if client_id != &ref_client_id {
            warn!(
                client_id = client_id,
                ref_client_id = ref_client_id,
                "Mismatching client ids"
            );
            return;
        }

        if status != &TransactionStatus::Disputed {
            warn!("Incorrect transaction status: {status:?}");
            return;
        }

        let Some(account) = self.accounts.get_mut(client_id) else {
            warn!("No account exists");
            return;
        };

        trace!("UPDATE_ACCT_FUNDS");
        account.held_funds -= *amount;
        account.available_funds += *amount;

        *status = TransactionStatus::Resolved;
        debug!(status = ?status, "UPDATE_TXN_STATUS");
    }

    /// Reverses a transaction, where funds that were previously held have now been withdrawn.
    /// decreases clients held funds and total funds by the amount previously disputed.
    /// This also freezes a clients account.
    #[tracing::instrument(skip(self))]
    pub fn handle_chargeback(&mut self, ref_txn_id: u32, ref_client_id: u16) {
        let Some(Transaction::Deposit {
            client_id,
            amount,
            status,
            ..
        }) = self.transaction_log.get_mut(&ref_txn_id)
        else {
            warn!("No Deposit transaction found with id {ref_txn_id}");
            return;
        };

        if *client_id != ref_client_id {
            warn!(
                client_id = client_id,
                ref_client_id = ref_client_id,
                "Mismatching client ids"
            );
            return;
        }

        if *status != TransactionStatus::Disputed {
            warn!("Transaction is not disputed");
            return;
        }

        // If we pass our initial checks, the account must be frozen if a withdrawal occurs.
        let Some(account) = self.accounts.get_mut(&client_id) else {
            warn!("No account exists");
            return;
        };

        trace!("FREEZE_ACCOUNT");
        account.freeze();

        trace!("UPDATE_ACCT_FUNDS");
        account.held_funds -= *amount;
        account.total_funds -= *amount;

        trace!("UPDATE_TXN_STATUS");
        *status = TransactionStatus::ChargedBack;
    }

    /// Deposits a provided amount into the account associated with the provided client_id.
    /// If the client id does not exist, we create a new account.
    #[tracing::instrument(skip(self, amount))]
    pub fn deposit_to_account(&mut self, client_id: u16, amount: Decimal) -> bool {
        let account = self
            .accounts
            .entry(client_id)
            .or_insert(ClientAccount::new(client_id));

        if account.frozen {
            error!("Account is frozen, unable to deposit");
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
    pub fn withdraw_from_account(&mut self, client_id: u16, amount: Decimal) -> bool {
        let Some(account) = self.accounts.get_mut(&client_id) else {
            error!("Account does not exist");
            return false;
        };

        if account.frozen {
            error!("Account is frozen");
            return false;
        }

        if account.available_funds - amount < dec!(0.0) {
            warn!("Insufficient available funds");
            return false;
        }

        account.available_funds -= amount;
        account.total_funds -= amount;

        true
    }

    /// print accounts of this [`AccountManager`] in a CSV format to stdout.
    pub fn print_accounts(&self) {
        let mut writer = csv::Writer::from_writer(std::io::stdout());

        for account in self.accounts.values() {
            if let Err(why) = writer.serialize(account.rounded()) {
                error!("Unable to serialize account: {why:?}");
                continue;
            };
        }

        if let Err(why) = writer.flush() {
            tracing::error!("Unable to flush csv writer: {why:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use crate::{
        accounts::{AccountManager, ClientAccount},
        transactions::{Transaction, TransactionStatus},
    };

    #[test]
    fn test_account_withdraw() {
        let mut act_mgr = AccountManager::default();
        let client_id = 1;
        act_mgr.deposit_to_account(client_id, dec!(10.0));
        let did_withdraw = act_mgr.withdraw_from_account(client_id, dec!(11.0));
        assert_eq!(did_withdraw, false);

        act_mgr.deposit_to_account(client_id, dec!(10.0));
        let did_withdraw = act_mgr.withdraw_from_account(client_id, dec!(9.0));
        assert!(did_withdraw);
    }

    #[test]
    fn test_account_deposit() {
        let client_id = 2;
        let mut act_mgr = AccountManager::default();
        let did_deposit = act_mgr.deposit_to_account(client_id, dec!(1.0));

        let _ = {
            let this = &mut act_mgr;
            this.accounts
                .entry(client_id)
                .or_insert(ClientAccount::new(client_id))
        };

        assert!(did_deposit);
    }

    #[test]
    fn test_dispute_then_chargeback() {
        let mut mgr = AccountManager::default();

        let txn_one = Transaction::Deposit {
            client_id: 1,
            transaction_id: 1,
            amount: dec!(100.0),
            status: TransactionStatus::Clean,
        };

        mgr.write_to_log(txn_one.clone());
        mgr.deposit_to_account(txn_one.client_id(), txn_one.amount());
        mgr.dispute_transaction(txn_one.id(), txn_one.client_id());
        mgr.handle_chargeback(txn_one.id(), txn_one.client_id());

        let Some(account) = mgr.accounts.get(&txn_one.client_id()) else {
            panic!("Account not found!");
        };

        assert_eq!(account.available_funds, dec!(0.0));
        assert_eq!(account.total_funds, dec!(0.0));
        assert_eq!(account.held_funds, dec!(0.0));
        assert!(account.frozen);
    }

    #[test]
    fn test_resolve() {
        let mut mgr = AccountManager::default();

        let txn_one = Transaction::Deposit {
            client_id: 1,
            transaction_id: 1,
            amount: dec!(100.0),
            status: TransactionStatus::Clean,
        };

        mgr.write_to_log(txn_one.clone());
        mgr.deposit_to_account(txn_one.client_id(), txn_one.amount());
        mgr.dispute_transaction(txn_one.id(), txn_one.client_id());
        mgr.resolve_transaction(txn_one.id(), txn_one.client_id());

        let Some(account) = mgr.accounts.get(&txn_one.client_id()) else {
            panic!("Account not found!");
        };

        assert_eq!(account.available_funds, dec!(100.0));
        assert_eq!(account.total_funds, dec!(100.0));
        assert_eq!(account.held_funds, dec!(0.0));
    }
}
