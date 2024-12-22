use std::fs;

pub fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("[{}] {}", record.level(), message)))
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        // .chain(fern::log_file("output.log").unwrap())
        // .chain(
        //     fs::OpenOptions::new()
        //         .write(true)
        //         .create(true)
        //         .truncate(true)
        //         .open("output.log")
        //         .unwrap(),
        // )
        .apply()?;
    Ok(())
}
