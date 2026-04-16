mod accounts;
mod cli;
mod parser;
mod transactions;

use std::fs::File;

use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::{accounts::AccountManager, cli::parse_cli_args, transactions::on_next_transaction};

fn main() -> anyhow::Result<()> {
    // Creating a structured logging setup to profile and trace
    // transaction_ids throughout the system.
    // By setting with_max_level to Level::TRACE, you will be able to see all trace messages.
    // Purposefully set to Level::INFO to just output accounts at the end.
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(Level::INFO)
        .with_line_number(true)
        .with_target(false)
        .with_file(true)
        .init();

    let args = parse_cli_args()?;

    // With this example, we parse an input file.
    // However, this could easily be abstracted out to a tcp server
    // by moving where the parsing happens.
    let file = File::open(&args.input_file_path)?;
    let mut reader = parser::build_csv_reader(file);

    // read the first row since we have headers in our example.
    // by calling this, we automatically consume the first row (headers).
    let _ = reader.headers()?;

    let mut account_manager = AccountManager::default();

    while let Some(next_record) = reader.records().next() {
        match next_record {
            Ok(record) => match parser::read_to_payment_record(&record, None) {
                Ok(payment_record) => on_next_transaction(payment_record, &mut account_manager),
                Err(why) => {
                    tracing::error!("{why:?}");
                    continue;
                }
            },
            Err(why) => {
                tracing::error!("{why:?}");
                continue;
            }
        }
    }

    account_manager.print_accounts();

    Ok(())
}
