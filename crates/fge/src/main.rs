use clap::Parser;
use fge::{app::execute_cli, cli::Cli};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    if let Err(err) = execute_cli(cli) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
