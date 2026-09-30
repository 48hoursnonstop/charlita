use anyhow::{Context, Result, bail};
fn main() {
    if let Err(error) = run() {
        eprintln!("Charlita engine: {error:#}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut portable = false;
    let mut explicit = None;
    let mut port = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--portable" => portable = true,
            "--headless" => {}
            "--data-dir" => {
                explicit = Some(std::path::PathBuf::from(
                    args.next().context("--data-dir needs a path")?,
                ))
            }
            "--port" => port = Some(args.next().context("--port needs a number")?.parse()?),
            "--version" => {
                println!("Charlita {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!(
                    "charlita-engine [--data-dir PATH] [--port PORT] [--portable]\nFor the desktop editor, run the Qt charlita executable."
                );
                return Ok(());
            }
            _ => bail!("Unknown option: {arg}"),
        }
    }
    let root = charlita::store::data_root(portable, explicit.as_deref())?;
    let (engine, runtime) = charlita::service::start(&root, port, None)?;
    println!("Charlita output: http://127.0.0.1:{}", engine.port);
    runtime.block_on(tokio::signal::ctrl_c())?;
    runtime.shutdown_timeout(std::time::Duration::from_secs(3));
    Ok(())
}
