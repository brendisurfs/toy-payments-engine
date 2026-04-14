use std::io::Read;

use csv::ByteRecord;
use serde::{Deserialize, Serialize};

/// The different actions we can have within our payments engine.
/// Note that Dispute, Resolve, and Chargeback do not have amounts,
/// as they reference the amount from the transaction ID (tx).
#[derive(Debug, Deserialize, Serialize)]
pub enum Transaction {
    Deposit {
        client: u16,
        tx_id: u64,
        amount: f32,
    },
    Withdrawal {
        client: u16,
        tx_id: u64,
        amount: f32,
    },
    Dispute {
        client: u16,
        tx_id: u64,
    },
    Resolve {
        client: u16,
        tx_id: u64,
    },
    Chargeback {
        client: u16,
        tx_id: u64,
    },
}

/// Parses an input Reader as a csv.
/// This should be able to take a generic stream of data,
/// # Errors
///
/// This function will return an error if .
pub fn build_csv_reader<R: Read>(input: R) -> anyhow::Result<()> {
    let reader = csv::Reader::from_reader(input);
    unimplemented!("Implement csv parser to stream")
}

pub fn parse_row_to_transaction(row: ByteRecord) {}

#[cfg(test)]
mod tests {
    use crate::parser::build_csv_reader;

    #[test]
    fn test_csv_reader_builder() {
        let data = r"
            city,country,pop
            Boston,United States,4628910
            Concord,United States,42695
        ";
        let _ = build_csv_reader(data.as_bytes());
        assert!(false);
    }
}
