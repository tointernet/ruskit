mod consumers;

use configs::{Configs, Empty};
use configs_builder::ConfigBuilder;
pub use consumers::todo;
use health_http_server::server::TinyHTTPServer;
use health_readiness::HealthReadinessServiceImpl;
use lapin::Connection;
use messaging::dispatcher::{Dispatcher, DispatcherDefinition};
use opentelemetry::{global, metrics::Observer};
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
        .trace()
        .metric()
        .rabbitmq()
        .build::<Empty>()
        .await?;

    traces::provider::init(&cfg)?;
    metrics::provider::init(&cfg)?;

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
    let meter = global::meter("rmq-consumer");
    let health_counter = meter
        .u64_observable_counter("rmq_consumer_health")
        .with_description("RMQ Consumer Health")
        .init();
    let callback = move |obs: &dyn Observer| {
        obs.observe_u64(&health_counter, 1, &[]);
    };

    match meter.register_callback(&[], callback) {
        Err(err) => {
            error!(error = err.to_string(), "error to register health counter");
            Err(err)
        }
        _ => Ok(()),
    }?;

    let heath_service = HealthReadinessServiceImpl::default().rabbitmq(rabbitmq_conn);

    Ok(TinyHTTPServer::new(cfgs).health_check(Arc::new(heath_service)))
}
