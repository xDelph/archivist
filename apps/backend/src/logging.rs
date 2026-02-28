use std::env;
use std::fs;
use std::sync::OnceLock;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static FILE_LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

fn is_service_env_development() -> bool {
    env::var("SERVICE_ENV")
        .map(|value| value.trim().eq_ignore_ascii_case("development"))
        .unwrap_or(false)
}

pub fn init_tracing() {
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
            return;
        }
    }

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .try_init()
        .ok();
}

#[cfg(test)]
mod tests {
    use super::is_service_env_development;

    #[test]
    fn service_env_development_detection_handles_case_and_spaces() {
        unsafe {
            std::env::set_var("SERVICE_ENV", "development");
        }
        assert!(is_service_env_development());

        unsafe {
            std::env::set_var("SERVICE_ENV", "  DeVeLoPmEnT  ");
        }
        assert!(is_service_env_development());
    }

    #[test]
    fn service_env_development_detection_rejects_other_values() {
        unsafe {
            std::env::set_var("SERVICE_ENV", "production");
        }
        assert!(!is_service_env_development());

        unsafe {
            std::env::remove_var("SERVICE_ENV");
        }
        assert!(!is_service_env_development());
    }
}
