use std::time::Duration;

use bytes::BytesMut;

use tokio_io::codec::Encoder;

use kafka::producer::{Producer, Record, Compression, RequiredAcks};

use zipkin;

use errors::{Error, Result};

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

pub struct KafkaCollector<E> {
    encoder: E,
    producer: Producer,
    topic: String,
    max_message_size: usize,
}

impl<'a, E> KafkaCollector<E>
    where E: Encoder<Item = zipkin::Span<'a>, Error = Error>
{
    pub fn new(config: KafkaConfig, encoder: E) -> Result<Self> {
        let producer = Producer::from_hosts(config.hosts).with_compression(config.compression)
            .with_ack_timeout(config.ack_timeout)
            .with_connection_idle_timeout(config.connection_idle_timeout)
            .with_required_acks(config.required_acks)
            .create()?;

        Ok(KafkaCollector {
            encoder: encoder,
            producer: producer,
            topic: config.topic,
            max_message_size: config.max_message_size,
        })
    }
}

impl<'a, E> zipkin::Collector<'a> for KafkaCollector<E>
    where E: Encoder<Item = zipkin::Span<'a>, Error = Error>
{
    type Error = Error;

    fn submit(&mut self, span: zipkin::Span<'a>) -> Result<()> {
        let key = span.name;
        let mut buf = BytesMut::with_capacity(self.max_message_size);

        self.encoder.encode(span, &mut buf)?;

        let record = Record::from_key_value(&self.topic, key, &buf[..]);

        self.producer.send(&record)?;

        Ok(())
    }
}
