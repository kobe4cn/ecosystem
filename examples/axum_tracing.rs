use std::time::{Duration, Instant};

use opentelemetry::{trace::TracerProvider, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{Config, RandomIdGenerator, Tracer},
    Resource,
};
use tokio::{net::TcpListener, time::sleep};
use tracing::{info, instrument, level_filters::LevelFilter, warn};

use tracing_subscriber::{
    fmt::{format::FmtSpan, Layer},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer as _,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let file_appender = tracing_appender::rolling::hourly("tmp/logs", "prefix.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let file = Layer::new()
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(non_blocking)
        .pretty()
        .with_filter(LevelFilter::INFO);

    let console = Layer::new()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .pretty()
        .with_filter(LevelFilter::INFO);

    let tracer = init_tracer()?;
    let otel_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(file)
        .with(console)
        .with(otel_layer)
        .init();

    let app = axum::Router::new().route("/", axum::routing::get(index_handler));
    let addr = "0.0.0.0:3000";
    info!("listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn index_handler() -> &'static str {
    sleep(Duration::from_millis(10)).await;
    let ret = long_task().await;
    info!(http.status = 200, "request completed");
    ret
}
#[instrument]
async fn long_task() -> &'static str {
    let start = Instant::now();
    sleep(Duration::from_millis(1000)).await;
    let elapsed = start.elapsed().as_millis() as u64;

    warn!(app.task_duration = elapsed, "long task completed");
    "hello kevin"
}

fn init_tracer() -> anyhow::Result<Tracer> {
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            Config::default()
                .with_id_generator(RandomIdGenerator::default())
                .with_max_attributes_per_span(64)
                .with_max_events_per_span(32)
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    "axum-tracing",
                )])),
        )
        .install_batch(runtime::Tokio)?;
    let tracer = tracer_provider.tracer_builder("axum-tracing").build();
    Ok(tracer)
}
