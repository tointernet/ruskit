use crate::errors::TracesError;
use configs::{Configs, DynamicConfigs, TraceExporterKind};
use tracing::{debug, error};

#[cfg(any(feature = "otlp", feature = "stdout"))]
use crate::exporters;

pub fn init<T>(cfg: &Configs<T>) -> Result<(), TracesError>
where
    T: DynamicConfigs,
{
    if !cfg.trace.enable {
        debug!("traces::init skipping trace export setup");
        return Ok(());
    }

    debug!("traces::init creating the tracer...");

    match cfg.trace.exporter {
        TraceExporterKind::Stdout => {
            #[cfg(feature = "stdout")]
            {
                exporters::stdout::install()
            }

            #[cfg(not(feature = "stdout"))]
            {
                debug!("stdout traces required to configure features = [stdout]");
                Err(TracesError::InvalidFeaturesError)
            }
        }
        TraceExporterKind::OtlpGrpc => {
            #[cfg(feature = "otlp")]
            {
                exporters::otlp_grpc::install(cfg)
            }

            #[cfg(not(feature = "otlp"))]
            {
                error!("otlp traces required to configure features = [otlp]");
                Err(TracesError::InvalidFeaturesError)
            }
        }
    }
}
