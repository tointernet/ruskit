use crate::errors::TracesError;
use configs::{Configs, DynamicConfigs};
use opentelemetry::{global, propagation::TextMapCompositePropagator};
use opentelemetry_otlp::{Protocol, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{
    propagation::{BaggagePropagator, TraceContextPropagator},
    runtime,
    trace::SdkTracerProvider,
};
use std::time::Duration;
use tonic::metadata::{Ascii, MetadataKey, MetadataMap};
use tracing::{debug, error};

pub fn install<T>(cfg: &Configs<T>) -> Result<(), TracesError>
where
    T: DynamicConfigs,
{
    let key: MetadataKey<Ascii> = match cfg.trace.header_access_key.clone().parse() {
        Ok(key) => key,
        Err(_) => {
            error!("failure to convert cfg.trace.header_key");
            MetadataKey::<Ascii>::from_bytes("api-key".as_bytes()).unwrap()
        }
    };

    let value = match cfg.trace.access_key.parse() {
        Ok(value) => Ok(value),
        Err(_) => {
            error!("failure to convert cfg.trace.header_value");
            Err(TracesError::ConversionError)
        }
    }?;

    let mut map = MetadataMap::with_capacity(2);
    map.insert(key, value);

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&cfg.trace.host)
        .with_protocol(Protocol::Grpc)
        .with_timeout(Duration::from_secs(cfg.trace.export_timeout))
        .with_metadata(map)
        .build()
        .map_err(|_| TracesError::ExporterProviderError)?;

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .build();

    global::set_tracer_provider(provider);

    global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()),
        Box::new(BaggagePropagator::new()),
    ]));

    debug!("traces::install otlp tracer installed");

    Ok(())
}
