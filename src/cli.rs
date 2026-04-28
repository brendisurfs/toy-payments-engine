use anyhow::anyhow;

/// The various inputs we can have.
/// For now, this is just a single input file,
/// however later we could implement a network stream path.
pub struct CliArgs {
    pub input_file_path: String,
}

/// Reads the incoming env arguments and builds a `CliArgs` struct.
///
/// # Errors
///
/// This function will return an error if no input file is given at the 1st element of the args.
pub fn parse_cli_args() -> anyhow::Result<CliArgs> {
    let input_file_path = std::env::args()
        .nth(1)
        .ok_or(anyhow!("No input file given"))?;

    Ok(CliArgs { input_file_path })
}
