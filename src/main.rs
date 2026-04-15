mod accounts;
mod cli;
mod parser;
mod transactions;

use std::fs::File;

use tracing::Level;

use crate::{accounts::AccountManager, cli::parse_cli_args, transactions::on_next_event};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_line_number(true)
        .with_target(false)
        .with_file(true)
        .init();

    let args = parse_cli_args()?;
    let file = File::open(&args.input_file_path)?;
    let mut reader = parser::build_csv_reader(file);

    // read the first row since we have headers in our example.
    // by calling this, we automatically consume the first row (headers).
    let _ = reader.headers()?;

    let mut account_manager = AccountManager::default();

    while let Some(Ok(record)) = reader.records().next() {
        let payment_record = parser::read_to_payment_record(&record, None)?;
        on_next_event(payment_record, &mut account_manager)?;
    }

    Ok(())
}
