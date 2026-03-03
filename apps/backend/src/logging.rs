use std::env;
use std::sync::OnceLock;

use tracing_subscriber::EnvFilter;

static PANIC_HOOK_SET: OnceLock<()> = OnceLock::new();

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
        eprintln!(
            r#"{{"level":"ERROR","target":"panic","message":"panic caught","location":"{}","payload":"{}"}}"#,
            location.replace('"', "'"),
            payload.replace('"', "'")
        );
        previous_hook(panic_info);
    }));
}

fn is_service_env_development() -> bool {
    env::var("SERVICE_ENV")
        .map(|value| value.trim().eq_ignore_ascii_case("development"))
        .unwrap_or(false)
}

fn default_env_filter(is_dev: bool) -> String {
    let base = if is_dev {
        "info,archivist=debug"
    } else {
        "info"
    };
    format!("{base},sqlx::query=warn,hyper_util::client::legacy::pool=warn,reqwest::retry=warn")
}

pub fn init_tracing() {
    // Local `vercel dev` may not expose shell env vars directly to Rust lambdas.
    // Load `.env` as a fallback source for SERVICE_ENV.
    let _ = dotenvy::dotenv();

    install_panic_hook();

    let is_dev = is_service_env_development();
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_env_filter(is_dev)));

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .try_init()
        .ok();
}

#[cfg(test)]
mod tests {
    use super::{default_env_filter, is_service_env_development};
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

    #[test]
    fn default_filter_reduces_library_noise() {
        let dev = default_env_filter(true);
        let prod = default_env_filter(false);
        assert!(dev.contains("sqlx::query=warn"));
        assert!(prod.contains("sqlx::query=warn"));
        assert!(dev.contains("hyper_util::client::legacy::pool=warn"));
        assert!(prod.contains("hyper_util::client::legacy::pool=warn"));
        assert!(dev.contains("reqwest::retry=warn"));
        assert!(prod.contains("reqwest::retry=warn"));
    }

    #[test]
    fn development_default_keeps_app_debug_visibility() {
        assert!(default_env_filter(true).starts_with("info,archivist=debug"));
    }
}
