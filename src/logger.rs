use indicatif::{ProgressBar, ProgressStyle};

pub const BLOCKY: &'static str = "█▛▌▖  ";

pub fn get_pb(total: usize, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!(
                "[{{elapsed_precise}}] {{prefix:.bold}}▕{{bar:.{}}}▏{{pos}}/{{len}}",
                "green"
            ))
            .progress_chars(BLOCKY),
    );
    pb.set_prefix(prefix.to_owned());
    pb
}

pub fn setup_logger() {
    let logger_dir = std::path::Path::new("logs");
    if !logger_dir.exists() {
        std::fs::create_dir(&logger_dir).unwrap();
    }
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .chain(fern::log_file(logger_dir.join("main.log")).unwrap())
        .apply()
        .unwrap();
}
