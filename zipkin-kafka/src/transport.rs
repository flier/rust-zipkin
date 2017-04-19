use std::time::Duration;
use std::marker::PhantomData;

use kafka::producer::{Producer, Record, Compression, RequiredAcks};

use zipkin_core::Transport;

use errors::Result;

pub struct KafkaConfig {
    pub hosts: Vec<String>,
    pub topic: String,
    pub max_message_size: usize,
    pub compression: Compression,
    pub ack_timeout: Duration,
    pub connection_idle_timeout: Duration,
    pub required_acks: RequiredAcks,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        KafkaConfig {
            hosts: vec![],
            topic: "zipkin".into(),
            max_message_size: 4096,
            compression: Compression::NONE,
            ack_timeout: Duration::from_secs(5),
            connection_idle_timeout: Duration::from_secs(30),
            required_acks: RequiredAcks::One,
        }
    }
}

impl KafkaConfig {
    pub fn new(hosts: &[String], topic: &str) -> Self {
        KafkaConfig {
            hosts: hosts.to_vec(),
            topic: topic.to_owned(),
            ..Default::default()
        }
    }
}

pub struct KafkaTransport<B, E> {
    producer: Producer,
    topic: String,
    phantom: PhantomData<(B, E)>,
}

impl<B, E> KafkaTransport<B, E> {
    pub fn new(config: KafkaConfig) -> Result<Self> {
        let producer = Producer::from_hosts(config.hosts)
            .with_compression(config.compression)
            .with_ack_timeout(config.ack_timeout)
            .with_connection_idle_timeout(config.connection_idle_timeout)
            .with_required_acks(config.required_acks)
            .create()?;

        Ok(KafkaTransport {
               producer: producer,
               topic: config.topic,
               phantom: PhantomData,
           })
    }
}

impl<B, E> Transport for KafkaTransport<B, E>
    where B: AsRef<[u8]> + Send + Sync,
          E: From<::kafka::Error> + Send + Sync
{
    type Buffer = B;
    type Output = ();
    type Error = E;

    fn send(&mut self, buf: &Self::Buffer) -> ::std::result::Result<Self::Output, Self::Error> {
        let record = Record::from_key_value(&self.topic, (), buf.as_ref());

        self.producer.send(&record)?;

        Ok(())
    }
}
