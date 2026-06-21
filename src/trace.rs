use chrono::{Local, TimeZone, Utc};
use std::sync::OnceLock;
use tracing_appender::rolling;
use tracing_subscriber::{self, layer::SubscriberExt};

const LOG_FILE_ONLY: bool = false;

static LOG_GUARDS: OnceLock<(
    tracing_appender::non_blocking::WorkerGuard,
    Option<tracing_appender::non_blocking::WorkerGuard>,
)> = OnceLock::new();
pub fn init_trace() {
    // Default to debug if PLAIN_LOGS undefined
    let plain_output =
        std::env::var("PLAIN_LOGS").unwrap_or_else(|_| "false".to_string()) == "true";

    // Configure file-based logging
    let file_appender = rolling::daily("./logs", "chess_rust.log");
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);

    // Set up environment filter for log levels
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"));

    // Create file logging layer
    let file_layer = tracing_subscriber::fmt::Layer::new()
        .with_writer(file_writer)
        .with_target(true);

    if !LOG_FILE_ONLY {
        // Configure stdout logging
        let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
        let stdout_layer = tracing_subscriber::fmt::Layer::new()
            .with_writer(stdout_writer)
            .with_target(true)
            .with_ansi(plain_output);

        // Combine file and stdout layers
        let subscriber = tracing_subscriber::Registry::default()
            .with(env_filter)
            .with(file_layer)
            .with(stdout_layer);

        // Keep guards alive
        LOG_GUARDS.get_or_init(|| (file_guard, Some(stdout_guard)));
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global subscriber");
    } else {
        // File-only logging
        let subscriber = tracing_subscriber::Registry::default()
            .with(env_filter)
            .with(file_layer);

        // Keep file guard alive
        LOG_GUARDS.get_or_init(|| (file_guard, None));
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global subscriber");
    }
}

pub fn trace_build_info() {
    let build_date = env!("BUILD_DATE", "BUILD_DATE not set during compilation");
    let git_commit = env!(
        "GIT_COMMIT_HASH",
        "GIT_COMMIT_HASH not set during compilation"
    );
    let timestamp = build_date
        .parse::<i64>()
        .expect("BUILD_DATE should be a valid timestamp");
    let utc_datetime = Utc
        .timestamp_opt(timestamp, 0)
        .single()
        .expect("Invalid timestamp");
    let local_datetime = utc_datetime.with_timezone(&Local);
    let formatted_date = local_datetime.format("%Y-%m-%d %H:%M:%S %Z").to_string();
    tracing::debug!("Build date: {}", formatted_date);
    tracing::debug!("Last commit hash: {}", git_commit);
}
