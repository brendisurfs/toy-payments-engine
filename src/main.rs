mod accounts;
mod cli;
mod parser;
mod transactions;

use std::fs::File;

use crate::{
    accounts::AccountManager, cli::parse_cli_args, parser::read_to_payment_record,
    transactions::handle_record,
};

use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

fn main() -> anyhow::Result<()> {
    // This creates a structured logging setup to profile and trace transaction_ids throughout the system.
    // By setting with_max_level to Level::TRACE, you will be able to see all trace messages.
    //
    // `.with_span_events` will show timings for each span.
    // This is useful for analyzing latency and processing time in performance critical scenarios.
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(Level::TRACE)
        .with_writer(std::io::stderr) // write to stderr instead of stdio to avoid noise.
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
            Ok(record) => {
                if let Ok(payment_record) = read_to_payment_record(&record, None) {
                    handle_record(payment_record, &mut account_manager);
                }
            }
            Err(why) => {
                tracing::error!("{why:?}");
                continue;
            }
        }
    }

    account_manager.print_accounts();

    Ok(())
}
