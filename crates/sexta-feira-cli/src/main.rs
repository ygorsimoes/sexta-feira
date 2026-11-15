//! CLI da Sexta-feira.

use std::io::Write;

use clap::{Parser, Subcommand};
use sexta_feira_core::Harness;
use tracing_subscriber::EnvFilter;

/// Sexta-feira: setup e motor do seu mordomo virtual.
#[derive(Debug, Parser)]
#[command(name = "sexta-feira", version, about)]
struct Cli {
    #[command(subcommand)]
    comando: Comando,
}

#[derive(Debug, Subcommand)]
enum Comando {
    /// Lista os harnesses previstos.
    Harnesses,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("SEXTA_FEIRA_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    match Cli::parse().comando {
        Comando::Harnesses => {
            let mut saida = std::io::stdout().lock();
            for harness in Harness::TODOS {
                writeln!(saida, "{harness}")?;
            }
        }
    }
    Ok(())
}
