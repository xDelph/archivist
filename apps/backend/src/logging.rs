use std::env;
use std::fs;
use std::io::Write;
use std::sync::OnceLock;

use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static FILE_LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();
static PANIC_HOOK_SET: OnceLock<()> = OnceLock::new();

fn append_raw_log_line(line: &str) {
    if fs::create_dir_all("./logs").is_err() {
        return;
    }
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("./logs/app.log")
    {
        let _ = writeln!(file, "{line}");
    }
}

fn install_panic_hook() {
    if PANIC_HOOK_SET.set(()).is_err() {
        return;
    }
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown".to_owned());
        let payload = if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            (*message).to_owned()
        } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            message.clone()
        } else {
            "non-string panic payload".to_owned()
        };
        append_raw_log_line(&format!(
            r#"{{"level":"ERROR","target":"panic","message":"panic caught","location":"{}","payload":"{}"}}"#,
            location.replace('"', "'"),
            payload.replace('"', "'")
        ));
        previous_hook(panic_info);
    }));
}

fn is_service_env_development() -> bool {
    env::var("SERVICE_ENV")
        .map(|value| value.trim().eq_ignore_ascii_case("development"))
        .unwrap_or(false)
}

pub fn init_tracing() {
    // Local `vercel dev` may not expose shell env vars directly to Rust lambdas.
    // Load `.env` as a fallback source for SERVICE_ENV.
    let _ = dotenvy::dotenv();

    install_panic_hook();

    let is_dev = is_service_env_development();
    let default_level = if is_dev { "trace" } else { "info" };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    if is_dev {
        if let Err(err) = fs::create_dir_all("./logs") {
            eprintln!("failed to create ./logs directory: {err}");
        } else {
            let file_appender = tracing_appender::rolling::never("./logs", "app.log");
            let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
            let _ = FILE_LOG_GUARD.set(guard);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_writer(file_writer),
                )
                .try_init()
                .ok();
            info!(
                service_env = "development",
                file = "./logs/app.log",
                "tracing initialized"
            );
            return;
        }
    }

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .try_init()
        .ok();
    info!(service_env = ?env::var("SERVICE_ENV").ok(), "tracing initialized");
}

#[cfg(test)]
mod tests {
    use super::is_service_env_development;
    use std::sync::{Mutex, OnceLock};

    static SERVICE_ENV_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn service_env_test_lock() -> std::sync::MutexGuard<'static, ()> {
        SERVICE_ENV_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("service env lock poisoned")
    }

    fn with_service_env<T>(value: Option<&str>, run: impl FnOnce() -> T) -> T {
        let _guard = service_env_test_lock();
        let previous = std::env::var("SERVICE_ENV").ok();
        match value {
            Some(next) => {
                // SAFETY: test-only environment mutation is protected by a process-wide mutex.
                unsafe { std::env::set_var("SERVICE_ENV", next) }
            }
            None => {
                // SAFETY: test-only environment mutation is protected by a process-wide mutex.
                unsafe { std::env::remove_var("SERVICE_ENV") }
            }
        }

        let output = run();

        match previous {
            Some(prev) => {
                // SAFETY: test-only environment mutation is protected by a process-wide mutex.
                unsafe { std::env::set_var("SERVICE_ENV", prev) }
            }
            None => {
                // SAFETY: test-only environment mutation is protected by a process-wide mutex.
                unsafe { std::env::remove_var("SERVICE_ENV") }
            }
        }
        output
    }

    #[test]
    fn service_env_development_detection_handles_case_and_spaces() {
        with_service_env(Some("development"), || {
            assert!(is_service_env_development());
        });
        with_service_env(Some("  DeVeLoPmEnT  "), || {
            assert!(is_service_env_development());
        });
    }

    #[test]
    fn service_env_development_detection_rejects_other_values() {
        with_service_env(Some("production"), || {
            assert!(!is_service_env_development());
        });
        with_service_env(None, || {
            assert!(!is_service_env_development());
        });
    }
}
