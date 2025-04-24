use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "drupal_ls")]
pub struct DrupalLspConfig {
    /// The file to pipe logs out to.
    #[clap(short, long)]
    pub file: Option<String>,

    /// The log level to use, defaults to INFO
    /// Valid values are: TRACE, DEBUG, INFO, WARN, ERROR
    #[clap(short, long, default_value = "INFO")]
    pub level: String,

    /// Uses stdio as the communication channel, will be as the default communication channel.
    #[clap(short, long)]
    pub stdio: bool,

    /// Use pipes (Windows) or socket files (Linux, Mac) as the communication channel.
    /// The pipe / socket file name is passed as the next arg or with --pipe=.
    /// Unsupported for now!
    #[clap(short, long)]
    pub pipe: Option<String>,

    /// Uses a socket as the communication channel. The port is passed as next arg or with --port=.
    #[clap(short, long)]
    pub socket: Option<u16>,

    /// The port to use for the socket connection.
    #[clap(short, long)]
    pub port: Option<u16>,
}
