use std::cmp;
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, AtomicIsize, Ordering};

pub trait Sampler {
    type Item;

    fn sample(&mut self, item: &Self::Item) -> bool;
}

/// Fixed rate sampling
#[derive(Debug)]
pub struct FixedRate<T> {
    pub sample_rate: usize,
    total_items: AtomicUsize,
    phantom: PhantomData<T>,
}

unsafe impl<T> Sync for FixedRate<T> {}

impl<T> FixedRate<T> {
    pub fn new(sample_rate: usize) -> Self {
        FixedRate {
            sample_rate: sample_rate,
            total_items: AtomicUsize::new(0),
            phantom: PhantomData,
        }
    }
}

impl<T> Default for FixedRate<T> {
    fn default() -> Self {
        FixedRate::new(1)
    }
}

impl<T> Sampler for FixedRate<T> {
    type Item = T;

    fn sample(&mut self, _: &Self::Item) -> bool {
        self.total_items.fetch_add(1, Ordering::Relaxed) % self.sample_rate == 0
    }
}

/// Rate limiting with token bucket
#[derive(Debug)]
pub struct RateLimit<T> {
    pub quantum: usize,
    pub capacity: usize,
    pub interval: Duration,
    tokens: AtomicIsize,
    ts: Instant,
    phantom: PhantomData<T>,
}

unsafe impl<T> Sync for RateLimit<T> {}

impl<T> RateLimit<T> {
    pub fn new(quantum: usize, capacity: usize, interval: Duration) -> Self {
        RateLimit {
            quantum: quantum,
            capacity: cmp::max(quantum, capacity),
            interval: interval,
            tokens: AtomicIsize::new(quantum as isize),
            ts: Instant::now(),
            phantom: PhantomData,
        }
    }

    pub fn per_second(quantum: usize, capacity: usize) -> Self {
        RateLimit::new(quantum, capacity, Duration::from_secs(1))
    }

    pub fn per_minute(quantum: usize, capacity: usize) -> Self {
        RateLimit::new(quantum, capacity, Duration::from_secs(60))
    }
}

impl<T> Sampler for RateLimit<T> {
    type Item = T;

    fn sample(&mut self, _: &Self::Item) -> bool {
        let remaining = self.tokens.fetch_sub(1, Ordering::Relaxed) - 1;

        if remaining >= 0 {
            true
        } else {
            let elapsed = self.ts.elapsed();

            if elapsed < self.interval {
                false
            } else {
                let tokens =
                    cmp::max(self.quantum as u64 *
                             (elapsed.as_secs() * 1000_000 + elapsed.subsec_nanos() as u64 / 1000) /
                             (self.interval.as_secs() * 1000_000 +
                              self.interval.subsec_nanos() as u64 / 1000),
                             self.capacity as u64) as isize - 1;

                if self.tokens.compare_and_swap(remaining, tokens, Ordering::Relaxed) == remaining {
                    self.ts = Instant::now();

                    true
                } else {
                    self.tokens.fetch_sub(1, Ordering::Relaxed) > 1
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::thread::{self, sleep};

    use super::*;

    #[test]
    fn fixed_rate() {
        let mut sampler = FixedRate::new(3);

        assert!(sampler.sample(&1));
        assert!(!sampler.sample(&2));
        assert!(!sampler.sample(&3));
        assert!(sampler.sample(&4));

        thread::spawn(move || {
                assert!(!sampler.sample(&5));
                assert!(!sampler.sample(&6));
                assert!(sampler.sample(&7));
                assert!(!sampler.sample(&8));
            })
            .join()
            .unwrap();
    }

    #[test]
    fn rate_limit() {
        let mut sampler = RateLimit::new(1, 2, Duration::from_millis(100));

        assert!(sampler.sample(&1));
        assert!(!sampler.sample(&2));

        sleep(Duration::from_millis(250));

        assert!(sampler.sample(&1));
        assert!(sampler.sample(&2));
        assert!(!sampler.sample(&3));
    }
}