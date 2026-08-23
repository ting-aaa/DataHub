use std::{env, fs};

use anyhow::{Context, Result};
use wit_component::ComponentEncoder;

fn main() -> Result<()> {
    let mut arguments = env::args().skip(1);
    let input = arguments.next().context("missing core Wasm input path")?;
    let output = arguments.next().context("missing component output path")?;
    if arguments.next().is_some() {
        anyhow::bail!("expected exactly an input and output path");
    }
    let module = fs::read(&input).with_context(|| format!("failed to read {input}"))?;
    let component = ComponentEncoder::default()
        .module(&module)
        .context("failed to read embedded WIT metadata")?
        .validate(true)
        .encode()
        .context("failed to encode component")?;
    fs::write(&output, component).with_context(|| format!("failed to write {output}"))?;
    Ok(())
}
