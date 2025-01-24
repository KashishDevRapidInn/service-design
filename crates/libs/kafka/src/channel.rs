use anyhow::Context;
use flume::Sender;
use rdkafka::message::ToBytes;

use crate::setup::KafkaTopic;


pub struct KafkaMessage<T: ToBytes> {
    pub message: T,
    pub topic: String,
}

impl<T: ToBytes> KafkaMessage<T> {
    fn new(message: T, topic: String) -> Self {
        Self {
            message,
            topic
        }
    }
}

pub async fn push_to_broker<T>(
    kafka_producer: &Sender<KafkaMessage<String>>,
    message: &T,
) -> Result<(), anyhow::Error>
where
    T: Sized + serde::ser::Serialize + KafkaTopic,
{
    let value = serde_json::to_string(&message).context("serialization error")?;
    let kafka_message = KafkaMessage::new(
        value,
        message.topic_name().to_string(),
    );
    match kafka_producer.send_async(kafka_message).await {
        Err(err) => {
            tracing::error!("flume send error :{:?}", err);
        }
        _ => {}
    }
    Ok(())
}
