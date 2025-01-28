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
}
