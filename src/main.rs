mod accounts;
mod cli;
mod parser;
mod transactions;

use std::{fs::File, rc::Rc};

use tracing::Level;

use crate::{accounts::AccountManager, cli::parse_cli_args, transactions::on_transaction};

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

    let mut account_manager = AccountManager::new();

    while let Some(Ok(record)) = reader.records().next() {
        let txn = parser::read_record_to_transaction(&record, None)?;

        // we wrap the transaction in an Rc so that we arent
        // cloning potentially a large enum variant.
        // This allows us to share across the tx_log without our memory usage
        // growing unruly in the case of a potentially long running system.
        // This does come with a slight performance overhead, though not in any meaningful way
        // here, but if this were a production system, this would possibly be changed.
        // For now, we are focusing on resource usage rather than raw hotpath performance.
        let txn = Rc::new(txn);
        on_transaction(txn, &mut account_manager)?;
    }

    Ok(())
}
