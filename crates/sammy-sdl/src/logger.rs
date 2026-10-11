//! The logger.

/// Logs to stderr, and on the Vita also to `log.txt` in the game's storage (opened per
/// line, so nothing is lost if the app is killed).
pub struct Log {
    file: Option<std::sync::Mutex<std::path::PathBuf>>,
}

impl log::Log for Log {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            let line = format!("{} {}: {}", r.level(), r.target(), r.args());
            eprintln!("{line}");
            if let Some(path) = &self.file
                && let Ok(path) = path.lock()
                && let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(&*path)
            {
                use std::io::Write;
                let _ = writeln!(f, "{line}");
            }
        }
    }
    fn flush(&self) {}
}

/// Logs at info level (on the Vita also to `log.txt`, emptied at start).
pub fn install() {
    let file = if cfg!(target_os = "vita") {
        let storage = sammy_platform::Storage::user();
        storage.dir().map(|d| {
            let path = d.join("log.txt");
            let _ = std::fs::write(&path, b"");
            path
        })
    } else {
        None
    };
    let logger: &'static Log = Box::leak(Box::new(Log { file: file.map(std::sync::Mutex::new) }));
    let _ = log::set_logger(logger).map(|_| log::set_max_level(log::LevelFilter::Info));
}
