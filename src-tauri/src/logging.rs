use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init(log_dir: PathBuf) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    std::fs::create_dir_all(&log_dir).ok()?;
    let appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "minipaste.log");
    let (nb, guard) = tracing_appender::non_blocking(appender);
    let env =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(nb).with_ansi(false))
        .with(env)
        .init();
    Some(guard)
}

pub fn install_panic_handler(log_path_hint: PathBuf) {
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "panic", "{}", info);
        let _ = std::fs::write(
            log_path_hint
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("LAST_CRASH"),
            info.to_string(),
        );
    }));
}
