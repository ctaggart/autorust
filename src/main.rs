mod cli;

use autorust_codegen::{run, Result};
use cli::config_try_new;

fn main() -> Result<()> {
    let config = config_try_new()?;
    run(config)?;
    Ok(())
}
