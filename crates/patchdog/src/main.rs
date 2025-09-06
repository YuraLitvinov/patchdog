use crate::analyzer::init_analyzer;
use crate::cli::cli_patch_to_agent;
use clap::Parser;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::BatchSpanProcessor;
use opentelemetry_sdk::trace::SdkTracerProvider;
use rust_parsing::error::ErrorBinding;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
pub mod analyzer;
pub mod binding;
pub mod cli;
#[cfg(test)]
pub mod tests;

/// The main entry point of the application, executed asynchronously. This function parses command-line arguments, sets up tracing for debugging if enabled, initializes the code analyzer, and loads environment variables.
/// It then delegates the core logic of processing patches and interacting with an AI agent to the `cli_patch_to_agent` function. The `tokio::main` attribute allows it to run asynchronous code.
///
/// # Returns
///
/// A `Result<(), ErrorBinding>` indicating the overall success or failure of the application's execution.
#[tokio::main]
async fn main() -> Result<(), ErrorBinding> {
    let commands = crate::cli::Mode::parse();
    if commands.enable_debug {
        setup_tracing();
    }
    let analyzer_data = init_analyzer();
    dotenv::dotenv().ok();
    cli_patch_to_agent(analyzer_data, commands).await?;
    Ok(())
}

/// Configures and initializes the application's tracing and logging infrastructure. This function sets up an OpenTelemetry (OTLP) exporter with a Tonic gRPC client for span processing, allowing for distributed tracing.
/// It also configures the `tracing-subscriber` to handle log filtering based on environment variables (RUST_LOG) or default settings, ensuring that telemetry and logs are properly collected and displayed for debugging and monitoring.
///
/// # Returns
///
/// An `SdkTracerProvider` instance, which is the root of the OpenTelemetry tracing system, allowing for the creation and management of spans.
fn setup_tracing() -> SdkTracerProvider {
    // Initialize OTLP exporter using gRPC (Tonic)
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to create OTLP exporter");

    let resource = Resource::builder().with_service_name("Patchdog").build();

    // Since BatchSpanProcessor and BatchSpanProcessorAsyncRuntime are not compatible with each other
    // we just create TracerProvider with different span processors
    let tracing_provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter).build())
        .with_resource(resource)
        .build();

    let targets_with_level =
        |targets: &[&'static str], level: LevelFilter| -> Vec<(&str, LevelFilter)> {
            // let default_log_targets: Vec<(String, LevelFilter)> =
            targets.iter().map(|t| ((*t), level)).collect()
        };

    tracing_subscriber::registry()
        // Telemetry filtering
        .with(
            tracing_opentelemetry::OpenTelemetryLayer::new(tracing_provider.tracer("embucket"))
                .with_level(true)
                .with_filter(filter_fn(|_| true)),
        )
        // Logs filtering
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_filter(match std::env::var("RUST_LOG") {
                    Ok(val) => match val.parse::<Targets>() {
                        // env var parse OK
                        Ok(log_targets_from_env) => log_targets_from_env,
                        Err(err) => {
                            eprintln!("Failed to parse RUST_LOG: {err:?}");
                            Targets::default().with_default(LevelFilter::DEBUG)
                        }
                    },
                    // No var set: use default log level INFO
                    _ => Targets::default()
                        .with_targets(targets_with_level(
                            // disable following targets:
                            &["tower_sessions", "tower_sessions_core", "tower_http"],
                            LevelFilter::OFF,
                        ))
                        .with_default(LevelFilter::INFO),
                }),
        )
        .init();

    tracing_provider
}
