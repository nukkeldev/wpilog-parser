use std::{fs::File, time::Instant};

use memmap2::Mmap;
use wpilog_parser::read_only;

fn main() {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::Level::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    let file = File::open("examples/parsing/Log_24-04-06_13-28-45_e5.wpilog").unwrap();
    let map = unsafe { Mmap::map(&file).unwrap() };

    let start = Instant::now();
    let log = read_only::WPILog::parse(&map).unwrap();

    println!("Parsed log ({:?})", start.elapsed());

    let timestamp = &log["/Timestamp"];

    // println!("{timestamp:?}");
}
