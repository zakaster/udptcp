use std::sync::mpsc;

pub struct Xlogger {
    tx: mpsc::Sender<String>,
}

impl log::Log for Xlogger {
    // not very helpful
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // true // log all (global logger)

        // we only want logs from our create, not the logs from eframe or egui
        metadata.target().starts_with("udptcp")
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let line = format!(
                "{} [{:16.16}][{:03}][{:5}]\t{}",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                record.file().unwrap_or_default(),
                record.line().unwrap_or_default(),
                record.level(),
                record.args()
            );

            // send the formatted line via channel
            let _ = self.tx.send(line);
        }
    }

    // not very helpful
    fn flush(&self) {}
}

impl Xlogger {
    /// only need to call init() once and we can
    /// use it everywhere inside create (answer from GPT)
    pub fn init() -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel();
        let logger = Xlogger { tx };
        log::set_boxed_logger(Box::new(logger)).expect("failed");
        log::set_max_level(log::LevelFilter::Trace);
        rx
    }
}

#[allow(unused)]
/// deprecated, this is the initial implementation
/// now we use log trait
///
/// format string into lines for scrollarea (log)
/// in format of timestamp [module][level], where levels include
///     - INFO
///     - ERR
///
/// examples
/// ```
/// log_line!("SYS", "INFO", "--- new session ---");
/// log_line!("CONN", format!("{:?}", self.connection));
/// ```
macro_rules! log_line {
    ($module:expr, $level:expr, $msg:expr) => {
        format!(
            "{} [{:5}][{:5}]\t{}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            $module,
            $level,
            $msg,
        )
    };
}
