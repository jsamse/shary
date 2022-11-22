use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

pub fn initialize() -> Result<()> {
    color_eyre::install()?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .wrap_err("setting default tracing subscriber failed")?;
    Ok(())
}
