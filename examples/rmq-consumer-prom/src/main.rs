mod consumers;

use configs::{Configs, Empty};
use configs_builder::ConfigBuilder;
pub use consumers::todo;
use health_http_server::server::TinyHTTPServer;
use health_readiness::HealthReadinessServiceImpl;
use lapin::Connection;
use messaging::dispatcher::{Dispatcher, DispatcherDefinition};
use opentelemetry::global;
use rabbitmq::{
    channel,
    dispatcher::RabbitMQDispatcher,
    exchange::ExchangeDefinition,
    queue::{QueueBinding, QueueDefinition},
    topology::{AmqpTopology, Topology},
};
use std::{error::Error, sync::Arc};
use tracing::error;

const QUEUE_NAME: &str = "my-rmq-queue";
const EXCHANGE_NAME: &str = "my-rmq-exchange";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cfgs = environment_setup().await?;

    let (rabbitmq_conn, rabbitmq_dispatcher) = rabbitmq_setup(&cfgs).await?;

    let health_server = health_http_server_setup(&cfgs, rabbitmq_conn)?;

    let meter = global::meter("rmq-consumer");
    let simple_counter = meter
        .u64_counter("simple_counter")
        .with_description("Simple counter")
        .init();
    simple_counter.add(1, &[]);

    let _observable = meter
        .u64_observable_counter("observable_counter")
        .with_description("Simple Observable Counter")
        .with_callback(|observer| observer.observe(1, &[]))
        .init();

    match tokio::join!(
        health_server.start(),
        rabbitmq_dispatcher.consume_blocking_single()
    ) {
        (Err(err), _) => {
            error!(error = err.to_string(), "error");
            panic!("{:?}", err)
        }
        (Ok(_), Err(err)) => {
            error!(error = err.to_string(), "error");
            panic!("{:?}", err)
        }
        _ => Ok(()),
    }
}

async fn environment_setup<'cfg>() -> Result<Configs<Empty>, Box<dyn Error>> {
    let cfg = ConfigBuilder::new()
        .health()
        .trace()
        .metric()
        .rabbitmq()
        .build::<Empty>()
        .await?;

    traces::provider::init(&cfg)?;

    Ok(cfg)
}

async fn rabbitmq_setup(
    cfg: &Configs<Empty>,
) -> Result<(Arc<Connection>, RabbitMQDispatcher), Box<dyn Error>> {
    let (conn, channel) = channel::new_amqp_channel(cfg).await?;

    let queue = QueueDefinition::new(QUEUE_NAME)
        .durable()
        .with_dlq()
        .with_retry(18_000, 3);

    let handler = todo::TodoConsumer::new();

    let definition = DispatcherDefinition::new(QUEUE_NAME, &format!("{}", todo::TodoMessage {}));
    let dispatcher = RabbitMQDispatcher::new(channel.clone(), vec![queue.clone()])
        .register(&definition, handler);

    AmqpTopology::new(channel.clone())
        .exchange(&ExchangeDefinition::new(EXCHANGE_NAME).direct().durable())
        .queue(&queue)
        .queue_binding(
            &QueueBinding::new(QUEUE_NAME)
                .exchange(EXCHANGE_NAME)
                .routing_key(&format!("{}-{}", QUEUE_NAME, EXCHANGE_NAME)),
        )
        .install()
        .await?;

    Ok((conn, dispatcher))
}

fn health_http_server_setup(
    cfgs: &Configs<Empty>,
    rabbitmq_conn: Arc<Connection>,
) -> Result<TinyHTTPServer, Box<dyn Error>> {
    let registry = metrics::provider::init(cfgs)?;
    let registry = registry.unwrap();

    let meter = global::meter("rmq-consumer");
    let _health_counter = meter
        .u64_observable_counter("rmq_consumer_health")
        .with_description("RMQ Consumer Health")
        .with_callback(|observer| observer.observe(1, &[]))
        .init();

    let heath_service = HealthReadinessServiceImpl::default().rabbitmq(rabbitmq_conn);

    Ok(TinyHTTPServer::new(cfgs)
        .metrics_registry(registry)
        .health_check(Arc::new(heath_service)))
}
