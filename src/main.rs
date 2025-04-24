mod opts;
mod parser;
mod server;
mod document_store;
mod documentation;
mod utils;

use std::fs::File;
use std::io::stderr;

use anyhow::Result;
use clap::Parser;
use structured_logger::json::new_writer;
use structured_logger::Builder;

use self::opts::DrupalLspConfig;
use self::server::start_lsp;

#[tokio::main]
async fn main() -> Result<()> {
    let config = DrupalLspConfig::parse();

    let mut builder = Builder::with_level(&config.level);

    if let Some(file) = &config.file {
        let log_file = File::options()
            .create(true)
            .append(true)
            .open(file)
            .unwrap();

        builder = builder.with_target_writer("*", new_writer(log_file));
    } else {
        builder = builder.with_target_writer("*", new_writer(stderr()))
    }

    builder.init();
    log::trace!("log options: {:?}", config);

    match start_lsp(config).await {
        Ok(_) => (),
        Err(error) => log::error!("An unexpected error happened: {:?}", error),
    };

    Ok(())
}
