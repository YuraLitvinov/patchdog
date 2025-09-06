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
/// The main asynchronous entry point of the application, marked with `#[tokio::main]`.
/// This function is responsible for initializing the application's environment, including parsing command-line arguments, setting up optional debug tracing, and loading environment variables from a `.env` file.
/// It then delegates the core business logic to `cli_patch_to_agent`, passing the initialized analyzer data and parsed commands.
///
/// # Returns
///
/// A `Result<(), ErrorBinding>`:
/// - `Ok(())`: Indicates successful execution of the application's main logic.
/// - `Err(ErrorBinding)`: Signifies an error during setup or the execution of `cli_patch_to_agent`.
/// The main entry point for the application, marked with `#[tokio::main]` for asynchronous execution.
/// It initializes dotenv for environment variables and tracing_subscriber for logging.
/// The core logic is delegated to `cli_patch_to_agent()`.
///
/// # Returns
///
/// An `Ok(())` on successful completion of the `cli_patch_to_agent` function.
/// An `ErrorBinding` if any error occurs during environment setup or the `cli_patch_to_agent` execution.
//Accepts relative path from inside folder
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

/// Sets up and configures the application's tracing and logging infrastructure using OpenTelemetry and `tracing-subscriber`.
/// It initializes an OTLP exporter to send telemetry data via gRPC and creates a `SdkTracerProvider` with a `BatchSpanProcessor` for efficient span processing.
/// Additionally, it configures two `tracing-subscriber` layers: one for OpenTelemetry telemetry and another for formatted console logs, allowing log levels to be controlled via the `RUST_LOG` environment variable and disabling specific noisy targets.
///
/// # Returns
///
/// An `SdkTracerProvider`: The configured OpenTelemetry `SdkTracerProvider` instance, which can be used to create new tracers.
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
