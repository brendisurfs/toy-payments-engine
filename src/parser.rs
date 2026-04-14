use std::io::Read;

use csv::StringRecord;
use serde::{Deserialize, Serialize};
use tracing::error;

const CSV_HEADERS: [&str; 4] = ["type", "client", "tx", "amount"];

/// Since the csv crate doesnt seem to allow for tagged enum variants,
/// we need to implement our own row struct that can be parsed.
#[derive(Debug, Deserialize)]
struct RawRow {
    #[serde(rename = "type")]
    tx_kind: String,
    client: u16,
    tx: u64,
    amount: Option<f32>,
}

/// The different actions we can have within our payments engine.
/// Note that Dispute, Resolve, and Chargeback do not have amounts,
/// as they reference the amount from the transaction ID (tx).
#[derive(Debug, Deserialize, PartialEq)]
#[serde(try_from = "RawRow", rename_all = "lowercase")]
pub enum Transaction {
    Deposit { client: u16, tx: u64, amount: f32 },
    Withdrawal { client: u16, tx: u64, amount: f32 },
    Dispute { client: u16, tx: u64 },
    Resolve { client: u16, tx: u64 },
    Chargeback { client: u16, tx: u64 },
}
impl TryFrom<RawRow> for Transaction {
    type Error = String;
    fn try_from(value: RawRow) -> Result<Self, Self::Error> {
        match value.tx_kind.trim() {
            "deposit" => {
                let amount = value.amount.ok_or("Deposit requires an amount")?;
                Ok(Transaction::Deposit {
                    client: value.client,
                    tx: value.tx,
                    amount,
                })
            }
            "withdrawal" => {
                let amount = value.amount.ok_or("Deposit requires an amount")?;
                Ok(Transaction::Withdrawal {
                    client: value.client,
                    tx: value.tx,
                    amount,
                })
            }
            "dispute" => Ok(Transaction::Dispute {
                client: value.client,
                tx: value.tx,
            }),
            "resolve" => Ok(Transaction::Resolve {
                client: value.client,
                tx: value.tx,
            }),
            "chargeback" => Ok(Transaction::Chargeback {
                client: value.client,
                tx: value.tx,
            }),
            other => Err(format!("Unknown transaction type: {other:?}")),
        }
    }
}

/// Parses an input Reader as a csv.
/// This should be able to take a generic stream of data,
/// # Errors
///
/// This function will return an error if .
pub fn build_csv_reader_from_stream<R: Read>(input: R) -> csv::Reader<R> {
    csv::Reader::from_reader(input)
}

/// Reads a transaction row and parses it into a record.
/// We take in optional headers in case this is coming from a csv,
/// but also handling the case that this could come from a TCP stream.
///
/// # Errors
///
/// This function will return an error if the reader cannot read to the byte record,
/// or if the record cannot deserialize to a Transaction variant.
pub fn read_row_to_record<R: Read>(
    reader: &mut csv::Reader<R>,
    headers: Option<&StringRecord>,
) -> anyhow::Result<()> {
    // initialize a new byte record to read into.
    let mut record = StringRecord::new();
    reader.read_record(&mut record)?;
    tracing::debug!("{record:?}");

    let tx_type = record
        .get(0)
        .expect("failed to get first field of record")
        .trim();

    tracing::debug!("{tx_type}");

    let transaction = record
        .deserialize::<Transaction>(headers)
        .inspect_err(|why| error!("Failed to deserialize record: {why:?}"))?;
    let _ = dbg!(transaction);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use csv::StringRecord;
    use tracing::Level;

    use crate::parser::{CSV_HEADERS, Transaction, build_csv_reader_from_stream};

    #[test]
    fn test_csv_reader_builder() {
        tracing_subscriber::fmt()
            .with_max_level(Level::DEBUG)
            .with_line_number(true)
            .with_file(true)
            .init();

        let Ok(data) = File::open("./test.csv") else {
            panic!("test file unable to open");
        };

        let headers = StringRecord::from_iter(CSV_HEADERS.iter());
        let mut reader = build_csv_reader_from_stream(data);
        assert!(true);
    }

    #[test]
    fn test_row_parses() {
        let wanted_transaction = Transaction::Deposit {
            client: 1,
            tx: 1,
            amount: 1.0,
        };

        let record = StringRecord::from(vec!["deposit", "1", "1", "1.0"]);
        let headers = StringRecord::from_iter(CSV_HEADERS.iter());
        let parsed_transaction = record.deserialize::<Transaction>(Some(&headers));
        assert!(parsed_transaction.is_ok());

        // We can safely unwrap because we assert before this test.
        assert_eq!(parsed_transaction.unwrap(), wanted_transaction);
    }
}
