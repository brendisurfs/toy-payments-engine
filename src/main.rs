use std::fs::File;

use tracing::Level;

use crate::cli::parse_cli_args;

mod cli;
mod parser;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_line_number(true)
        .with_file(true)
        .init();

    let args = parse_cli_args()?;
    let file = File::open(&args.input_file_path)?;
    let mut reader = parser::build_csv_reader(file);

    // read the first row since we have headers in our example.
    // by calling this, we automatically consume the first row (headers).
    let _ = reader.headers()?;

    while let Some(Ok(record)) = reader.records().next() {
        let transaction = parser::read_row_to_record(&record, None)?;
        tracing::info!("{transaction:?}");
    }

    Ok(())
}
