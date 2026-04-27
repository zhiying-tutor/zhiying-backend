use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use deadpool_lapin::Pool;
use deadpool_lapin::lapin::{
    BasicProperties, ExchangeKind,
    options::{BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
};

use crate::error::{AppError, BusinessError};

pub const ROUTING_KEY_GENERATE: &str = "generate";

#[async_trait]
pub trait MessagePublisher: Send + Sync {
    async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &[u8],
    ) -> Result<(), AppError>;
}

pub struct LapinPublisher {
    pool: Pool,
}

impl LapinPublisher {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MessagePublisher for LapinPublisher {
    async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &[u8],
    ) -> Result<(), AppError> {
        let conn = self.pool.get().await.map_err(|err| {
            tracing::error!(error = %err, "failed to acquire rabbitmq connection");
            AppError::business(BusinessError::ServiceUnavailable)
        })?;

        let channel = conn.create_channel().await.map_err(|err| {
            tracing::error!(error = %err, "failed to open rabbitmq channel");
            AppError::business(BusinessError::ServiceUnavailable)
        })?;

        channel
            .confirm_select(Default::default())
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "failed to enable publisher confirms");
                AppError::business(BusinessError::ServiceUnavailable)
            })?;

        let props = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_delivery_mode(2);

        let confirm = channel
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                payload,
                props,
            )
            .await
            .map_err(|err| {
                tracing::error!(error = %err, exchange, routing_key, "failed to publish message");
                AppError::business(BusinessError::ServiceUnavailable)
            })?
            .await
            .map_err(|err| {
                tracing::error!(error = %err, exchange, routing_key, "publisher confirm failed");
                AppError::business(BusinessError::ServiceUnavailable)
            })?;

        if confirm.is_nack() {
            tracing::error!(exchange, routing_key, "broker nacked message");
            return Err(AppError::business(BusinessError::ServiceUnavailable));
        }

        Ok(())
    }
}

pub struct TopologyEntry<'a> {
    pub exchange: &'a str,
    pub queue_suffix: &'a str,
    pub routing_key: &'a str,
}

pub async fn declare_topology(
    pool: &Pool,
    entries: &[TopologyEntry<'_>],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn = pool.get().await?;
    let channel = conn.create_channel().await?;

    for entry in entries {
        channel
            .exchange_declare(
                entry.exchange,
                ExchangeKind::Direct,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        let queue_name = format!("{}.{}", entry.exchange, entry.queue_suffix);

        channel
            .queue_declare(
                &queue_name,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        channel
            .queue_bind(
                &queue_name,
                entry.exchange,
                entry.routing_key,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct PublishedMessage {
    pub exchange: String,
    pub routing_key: String,
    pub payload: Vec<u8>,
}

#[derive(Default)]
pub struct InMemoryPublisher {
    sent: Mutex<Vec<PublishedMessage>>,
    fail_next: Mutex<bool>,
}

impl InMemoryPublisher {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn fail_next(&self) {
        *self.fail_next.lock().unwrap() = true;
    }

    pub fn take(&self) -> Vec<PublishedMessage> {
        std::mem::take(&mut *self.sent.lock().unwrap())
    }

    pub fn find_by_exchange(&self, exchange: &str) -> Option<PublishedMessage> {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.exchange == exchange)
            .cloned()
    }

    pub fn len(&self) -> usize {
        self.sent.lock().unwrap().len()
    }
}

#[async_trait]
impl MessagePublisher for InMemoryPublisher {
    async fn publish(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &[u8],
    ) -> Result<(), AppError> {
        {
            let mut flag = self.fail_next.lock().unwrap();
            if *flag {
                *flag = false;
                return Err(AppError::business(BusinessError::ServiceUnavailable));
            }
        }
        self.sent.lock().unwrap().push(PublishedMessage {
            exchange: exchange.to_owned(),
            routing_key: routing_key.to_owned(),
            payload: payload.to_vec(),
        });
        Ok(())
    }
}
