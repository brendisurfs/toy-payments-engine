use std::io::Read;

use csv::{StringRecord, Trim};
use tracing::error;

use crate::transactions::Transaction;

/// Parses an input Reader as a csv.
/// This should be able to take a generic stream of data.
pub fn build_csv_reader<R: Read>(input: R) -> csv::Reader<R> {
    csv::ReaderBuilder::new().trim(Trim::All).from_reader(input)
}

/// Reads a transaction row and parses it into a record.
/// We take in optional headers in case this is coming from a csv,
/// but also handling the case that this could come from a TCP stream.
///
/// # Errors
///
/// This function will return an error if the reader cannot read to the byte record,
/// or if the record cannot deserialize to a Transaction variant.
pub fn read_record_to_transaction(
    record: &StringRecord,
    headers: Option<&StringRecord>,
) -> anyhow::Result<Transaction> {
    let transaction = record
        .deserialize::<Transaction>(headers)
        .inspect_err(|why| error!("Failed to deserialize record: {why:?}"))?;

    Ok(transaction)
}

#[cfg(test)]
mod tests {
    use crate::transactions::Transaction;
    use std::fs::File;

    use csv::StringRecord;

    use crate::parser::build_csv_reader;

    #[test]
    fn test_csv_reader_builder() {
        let data = File::open("./test.csv").expect("test file unable to open");
        let reader = build_csv_reader(data);
        assert!(true);
    }

    #[test]
    fn test_row_parses() {
        let wanted_transaction = Transaction::Deposit {
            client_id: 1,
            tx: 1,
            amount: 1.0,
        };

        let record = StringRecord::from(vec!["deposit", "1", "1", "1.0"]);
        let headers = StringRecord::from_iter(["type", "client", "tx", "amount"].iter());
        let parsed_transaction = record.deserialize::<Transaction>(Some(&headers));
        assert!(parsed_transaction.is_ok());

        // We can safely unwrap because we assert before this test.
        assert_eq!(parsed_transaction.unwrap(), wanted_transaction);
    }
}
