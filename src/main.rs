mod app;
mod storage;

fn main() -> cosmic::iced::Result {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let _ = tracing_log::LogTracer::init();
    tracing::info!("Starting ClipManager applet");
    cosmic::applet::run::<app::AppModel>(())
}
