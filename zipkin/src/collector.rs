use std::time::{SystemTime, Duration};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, Receiver, SendError, channel};

use futures::Future;
use futures::future::{BoxFuture, ok, loop_fn};
use futures_cpupool::{CpuPool, CpuFuture};

use span::Span;

pub trait Collector<'a> {
    type Error;

    fn submit(&self, span: Span<'a>) -> Result<(), Self::Error>;
}
/*
pub trait Transport<M> {
    fn send(&self, msg: M) -> Result<()>;
}

struct Batch<M, T> where T: Transport<M>{
    transport: Arc<Mutex<T>>,
    messages: Vec<M>,
}

impl <M, T> for Batch<M, T>where T: Transport<M> {
    fn send_message(&mut self) ->FutureResult<(Self, bool), Error> {
        if let Some(msg) = self.messages.pop() {

        }

        self.messages.is_empty()
    }
}

pub struct Dispatcher<M, T: Transport<M>> {
    cpu_pool: CpuPool,
    transport: Arc<Mutex<T>>,
    messages: Arc<Mutex<Vec<M>>>,

    /// the maximum batch size, after which a collect will be triggered.
    pub batch_size: usize,
    /// the maximum duration we will buffer traces before emitting them to the collector.
    pub batch_interval: Duration,
}

impl<M, T> Dispatcher<M, T>
    where  T: Transport<M>
{
    pub fn new(transport: Arc<Mutex<T>>) -> Self {
        Dispatcher {
            cpu_pool: CpuPool::new_num_cpus(),
            batch_size: 100,
            batch_interval: Duration::from_secs(1),
            transport: transport,
            messages: Arc::new(Mutex::new(vec![])),
            last_send_time: SystemTime::now(),
        }
    }

    pub fn dispatch(&mut self, msg: M) -> BoxFuture<(), Error> {
        if self.push_message(msg)? {
            self.send_messages()
        } else {
            ok(())
        }
    }

    fn push_message(&self, msg: M) -> Result<bool> {
        let messages = self.messages.lock()?;

        messages.push(msg);

        Ok(messages.len() > self.batch_size ||
           self.last_send_time.elapsed()? >= self.batch_interval)
    }

    fn send_messages(&self) -> BoxFuture<(), Error> {
        let send_in_batch = loop_fn(Batch {
            transport: self.transport.clone(),
            messages: {
                let messages = self.messages.lock()?;

                messages.drain(..).collect()
            },
        });

        self.cpu_pool
            .spawn_fn(|| {
                loop {
                    let state = state.lock()?;

                    if let Some(msg) = state.messages.pop() {
                        state.transport.send(msg)?;

                        state.last_send_time = SystemTime::now();
                    } else {
                        break;
                    }
                }

                Ok(())
            })
            .boxed()
    }
}
*/