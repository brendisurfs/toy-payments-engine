use std::io::Read;

use anyhow::bail;
use csv::{StringRecord, Trim};
use tracing::error;

use crate::transactions::{PaymentEvent, Transaction};

/// Parses an input Reader as a csv.
/// This should be able to take a generic stream of data.
pub fn build_csv_reader<R: Read>(input: R) -> csv::Reader<R> {
    csv::ReaderBuilder::new().trim(Trim::All).from_reader(input)
}

pub enum PaymentRecord {
    Transaction(Transaction),
    MutatingEvent(PaymentEvent),
}

/// Reads a transaction row and parses it into a record.
/// We take in optional headers in case this is coming from a csv,
/// but also handling the case that this could come from a TCP stream.
///
/// # Errors
///
/// This function will return an error if the reader cannot read to the byte record,
/// or if the record cannot deserialize to a Transaction variant.
pub fn read_to_payment_record(
    record: &StringRecord,
    headers: Option<&StringRecord>,
) -> anyhow::Result<PaymentRecord> {
    let Some(record_type) = record.get(0) else {
        bail!("No record type found at index 0");
    };

    match record_type {
        "deposit" | "withdrawal" => {
            let parsed_txn = record
                .deserialize::<Transaction>(headers)
                .inspect_err(|why| error!("Failed to deserialize record: {why:?}"))?;

            Ok(PaymentRecord::Transaction(parsed_txn))
        }

        "dispute" | "resolve" | "chargeback" => {
            let parsed_event = record
                .deserialize::<PaymentEvent>(headers)
                .inspect_err(|why| error!("Failed to deserialize record: {why:?}"))?;

            Ok(PaymentRecord::MutatingEvent(parsed_event))
        }

        other => bail!("Invalid transaction type found: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use crate::transactions::{Transaction, TransactionStatus};

    use csv::StringRecord;
    use rust_decimal_macros::dec;

    #[test]
    fn test_row_parses() {
        let wanted_transaction = Transaction::Deposit {
            amount: dec!(1.0),
            transaction_id: 1,
            client_id: 1,
            status: TransactionStatus::Clean,
        };

        let record = StringRecord::from(vec!["deposit", "1", "1", "1.0"]);
        let parsed_transaction = record.deserialize::<Transaction>(None);

        assert!(parsed_transaction.is_ok());

        assert_eq!(
            wanted_transaction,
            parsed_transaction.expect("failed to get parsed_transaction")
        );
    }
}
