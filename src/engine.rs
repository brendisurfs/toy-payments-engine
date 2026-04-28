use crate::{
    accounts::AccountManager,
    parser::PaymentRecord,
    transactions::{PaymentEvent, Transaction},
};

/// handles the next transaction event from our stream.
/// We match on our incoming event and calls the proper function to that event.
#[tracing::instrument(skip(manager, record), fields(txn_id = record.txn_id()))]
pub fn handle_record(record: PaymentRecord, manager: &mut AccountManager) {
    match record {
        PaymentRecord::Transaction(txn) => {
            match txn {
                Transaction::Deposit {
                    client_id, amount, ..
                } => {
                    manager.write_to_log(txn);
                    manager.deposit_to_account(client_id, amount)
                }
                Transaction::Withdrawal {
                    client_id, amount, ..
                } => manager.withdraw_from_account(client_id, amount),
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
