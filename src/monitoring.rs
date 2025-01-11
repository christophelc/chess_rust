pub mod debug;
pub mod version;

#[macro_export]
macro_rules! span_debug {
    ($module_name:expr) => {
        tracing::span!(
            tracing::Level::DEBUG,
            $module_name,
            app_version = crate::monitoring::version::version()
        )
    };
}

#[macro_export]
macro_rules! span_error {
    ($module_name:expr) => {
        tracing::span!(
            tracing::Level::ERROR,
            $module_name,
            app_version = crate::monitoring::version::version()
        )
    };
}
