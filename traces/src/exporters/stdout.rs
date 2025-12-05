use crate::errors::TracesError;
use opentelemetry::{global, propagation::TextMapCompositePropagator};
use opentelemetry_sdk::{
    propagation::{BaggagePropagator, TraceContextPropagator},
    trace::SdkTracerProvider,
};
use tracing::debug;

pub fn install() -> Result<(), TracesError> {
    let exporter = opentelemetry_stdout::SpanExporter::default();

    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();

    global::set_tracer_provider(provider);

    global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()),
        Box::new(BaggagePropagator::new()),
    ]));

    debug!("traces::install stdout tracer installed");

    Ok(())
}
