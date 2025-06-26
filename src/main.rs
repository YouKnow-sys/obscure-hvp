use clap::Parser;

mod commands;

fn main() -> anyhow::Result<()> {
    commands::Commands::parse().start()
}
