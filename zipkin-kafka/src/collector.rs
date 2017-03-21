use std::time::Duration;

use futures::Future;
use futures_cpupool::{CpuPool, CpuFuture};

use kafka;
use kafka::producer::{Producer, Record, Compression, RequiredAcks};

use zipkin;

use errors::{Error, Result};

pub struct KafkaConfig {
    pub hosts: Vec<String>,
    pub topic: String,
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
            compression: Compression::NONE,
            ack_timeout: Duration::from_secs(5),
            connection_idle_timeout: Duration::from_secs(30),
            required_acks: RequiredAcks::One,
        }
    }
}

pub struct KafkaCollector {
    cpu_pool: CpuPool,
    producer: Producer,
    topic: String,
}

impl KafkaCollector {
    pub fn new(config: &KafkaConfig) -> Result<Self> {
        let producer = Producer::from_hosts(config.hosts).with_compression(config.compression)
            .with_ack_timeout(config.ack_timeout)
            .with_connection_idle_timeout(config.connection_idle_timeout)
            .with_required_acks(config.required_acks)
            .create()?;

        Ok(KafkaCollector {
            cpu_pool: CpuPool::new_num_cpus(),
            producer: producer,
            topic: config.topic,
        })
    }
}

impl<'a> zipkin::Collector<'a> for KafkaCollector {
    type Error = Error;

    fn submit(&self,
              span: zipkin::Span<'a>)
              -> Box<Future<Item = zipkin::Span<'a>, Error = Self::Error>> {
        Box::new(self.cpu_pool.spawn_fn(|| Ok(span)))
    }
}
