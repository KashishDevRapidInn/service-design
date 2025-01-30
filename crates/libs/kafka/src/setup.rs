use std::time::Duration;

use flume::{Receiver, Sender};
use futures::StreamExt;
use rdkafka::{
    admin::{
        AdminClient, AdminOptions, NewTopic, TopicReplication
    },
    client::DefaultClientContext,
    config::{FromClientConfig, FromClientConfigAndContext},
    consumer::{Consumer, StreamConsumer},
    message::{OwnedMessage, ToBytes},
    producer::{FutureProducer, FutureRecord},
    types::RDKafkaErrorCode, ClientConfig
};

use crate::channel::KafkaMessage;

pub trait KafkaTopic {
    fn topic_name(&self) -> String;
}

pub async fn setup_kafka_sender(
    kafka_broker_url: &String,
    topics: &Vec<String>,
) -> Sender<KafkaMessage<String>> {
    let (tx, rx) = flume::unbounded();

    create_topics(kafka_broker_url, topics).await;
    let producer = create_producer(kafka_broker_url);

    tokio::spawn(produce(rx, producer));

    tx
}

pub async fn setup_kafka_receiver(
    kafka_broker_url: &String,
    topics: &Vec<String>,
    consumer_group: &String
) -> Receiver<OwnedMessage> {
    let (tx, rx) = flume::unbounded();

    let consumer = create_consumer(kafka_broker_url, consumer_group);
    tracing::info!("Topics subscription: {:?}", topics);
    consumer.subscribe(&topics.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .expect("Failed to subsribe to topics");

    tokio::spawn(consume(tx, consumer));

    rx
}

pub fn create_consumer(broker_url: &str, group: &str) -> StreamConsumer {
    
    match ClientConfig::new()
        .set("group.id", group)
        .set("bootstrap.servers", broker_url)
        .set("enable.auto.commit", "true")
        .set("auto.commit.interval.ms", "1000")
        .set("auto.offset.reset", "earliest")
        .set("session.timeout.ms", "6000")
        .create(){
        
        Ok(client) => {
            client
        }
        Err(e) => panic!(" kafka client error {:?}", e),
        
    }
}

async fn create_topics(url: &str, topics: &Vec<String>) {
    let admin_client: AdminClient<_> = create_admin_client(url);

    for topic in topics {
        tracing::info!("creating topic: {}", topic);
        let new_topic = NewTopic::new(topic, 3, TopicReplication::Fixed(1));
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(5)));
        let admin_res = admin_client
            .create_topics(&[new_topic], &opts)
            .await
            .expect("could not create the topic");

        for admin_result in admin_res {
            match admin_result {
                Ok(_) => {tracing::info!("created: {}", topic)}
                Err((topic, err)) => {
                    if !matches!(err, RDKafkaErrorCode::TopicAlreadyExists) {
                        panic!("{:?} -> {:?}", topic, err);
                    }

                    tracing::info!("already exists: {}", topic)
                }
            }
        }
    }
}

pub fn create_producer(broker_url: &str) -> FutureProducer {
    match ClientConfig::new()
        .set("bootstrap.servers", broker_url)
        .set("message.timeout.ms", "5000")
        .create()
    {
        Ok(client) => {
            // let metadata = client, check if the client is active
            client
        }
        Err(e) => panic!(" kafka client error {:?}", e),
    }
}

fn create_admin_client<T>(broker_url: &str) -> T
where
    T: FromClientConfigAndContext<DefaultClientContext> + FromClientConfig
{
    match ClientConfig::new()
        .set("bootstrap.servers", broker_url)
        .set("message.timeout.ms", "5000")
        .create()
    {
        Ok(client) => {
            // let metadata = client, check if the client is active
            client
        }
        Err(e) => panic!(" kafka client error {:?}", e),
    }
}

pub async fn consume(
    tx: Sender<OwnedMessage>,
    consumer: StreamConsumer
) {
    consumer
        .stream()
        .for_each_concurrent(Some(10), |message| {
            let message_sender = tx.clone();
            async move {
                match message {
                    Ok(msg) => {
                        tracing::info!("received message successfully");
                        if let Err(e) = message_sender.send(msg.detach()) {
                            tracing::error!("flume channel error : {:?}", e);
                        }
                    }
                    Err(err) => {
                        tracing::error!("consumer error : {:?}", err);
                    }
                }
            }
        })
        .await;
}

pub async fn produce<T: ToBytes + Clone + Send>(
    rx: Receiver<KafkaMessage<T>>,
    producer: FutureProducer,
) {
    rx.stream()
        .for_each(|msg| {
            let producer = producer.clone();
            async move {
                match producer
                    .send(
                        FutureRecord::<String, T>::to(&msg.topic)
                            .payload(&msg.message),
                        Duration::from_secs(0),
                    )
                    .await
                {
                    Ok(_) => {
                        tracing::info!("sucessfully delivered to kafka broker");
                    }
                    Err((err, msg)) => {
                        tracing::error!("kafka producer error : {:?}, message : {:?}", err, msg);
                    }
                }
            }
        })
        .await;
}
