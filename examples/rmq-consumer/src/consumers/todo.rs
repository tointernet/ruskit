use async_trait::async_trait;
use messaging::{
    errors::MessagingError,
    handler::{ConsumerHandler, ConsumerMessage},
};
use opentelemetry::Context;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};
use tracing::{error, info};

pub struct TodoConsumer {}

impl TodoConsumer {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

#[async_trait]
impl ConsumerHandler for TodoConsumer {
    async fn exec(&self, _ctx: &Context, msg: &ConsumerMessage) -> Result<(), MessagingError> {
        let received = match serde_json::from_slice::<TodoMessage>(&msg.data) {
            Ok(r) => Ok(r),
            Err(err) => {
                error!(
                    error = err.to_string(),
                    payload = format!("{:?}", msg.data),
                    "parsing error"
                );
                Err(MessagingError::SerializingError {})
            }
        }?;

        info!("received msg - {:?}", received);

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TodoMessage;

impl Display for TodoMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TodoMessage")
    }
}

#[cfg(test)]
mod tests {}
