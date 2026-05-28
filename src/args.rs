use std::path::PathBuf;

use clap::builder::styling::AnsiColor;
use clap::{Parser, builder};

#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    about,
    long_about = None,
    styles = get_styles(),
)]
pub struct MdwatchArgs {
    /// Path to the markdown file
    pub file: PathBuf,

    /// IP address to bind the server
    #[clap(short, long, default_value = "127.0.0.1")]
    pub ip: String,

    /// Bind to all interfaces (0.0.0.0)
    #[clap(long, conflicts_with = "ip")]
    pub public: bool,

    /// Port number to serve on (If not provided, a random port will be used)
    #[clap(short, long)]
    pub port: Option<u16>,
}

fn get_styles() -> clap::builder::Styles {
    builder::Styles::styled()
        .header(AnsiColor::on_default(AnsiColor::Cyan))
        .usage(AnsiColor::on_default(AnsiColor::Green).bold())
        .literal(AnsiColor::on_default(AnsiColor::Cyan))
        .placeholder(AnsiColor::on_default(AnsiColor::Yellow).italic())
        .error(AnsiColor::on_default(AnsiColor::Red).bold())
        .valid(AnsiColor::on_default(AnsiColor::BrightGreen))
        .invalid(AnsiColor::on_default(AnsiColor::BrightRed).bold())
}
