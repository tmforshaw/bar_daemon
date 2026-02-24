use std::time::Duration;

use tokio::{sync::mpsc, time::Instant};

#[async_trait::async_trait]
pub trait Trigger: Send {
    async fn wait(&mut self);
}

// Polled Trigger

pub struct IntervalTrigger {
    timer: tokio::time::Interval,
}

impl IntervalTrigger {
    #[must_use]
    pub fn new(period: Duration) -> Self {
        Self {
            timer: tokio::time::interval(period),
        }
    }
}

#[async_trait::async_trait]
impl Trigger for IntervalTrigger {
    async fn wait(&mut self) {
        self.timer.tick().await;
    }
}

// Event-driven Trigger

pub struct EventTrigger {
    rx: mpsc::Receiver<()>,
}

impl EventTrigger {
    #[must_use]
    pub const fn new(rx: mpsc::Receiver<()>) -> Self {
        Self { rx }
    }
}

#[async_trait::async_trait]
impl Trigger for EventTrigger {
    async fn wait(&mut self) {
        // Wait until event is received
        let _ = self.rx.recv().await;
    }
}

// Debounced event-driven Trigger
pub struct DebouncedEventTrigger {
    rx: tokio::sync::mpsc::Receiver<()>,
    debounce: Duration,
}

impl DebouncedEventTrigger {
    #[must_use]
    pub const fn new(rx: tokio::sync::mpsc::Receiver<()>, debounce: Duration) -> Self {
        Self { rx, debounce }
    }
}

#[async_trait::async_trait]
impl Trigger for DebouncedEventTrigger {
    async fn wait(&mut self) {
        let _ = self.rx.recv().await; // wait for first signal
        let start = Instant::now();
        loop {
            tokio::select! {
                _ = self.rx.recv() => {
                    // keep looping, reset timer for new signals
                }
                () = tokio::time::sleep_until(start + self.debounce) => break,
            }
        }
    }
}

// Hybrid polling and event-driven Trigger

pub struct HybridTrigger<T: Trigger> {
    event: T,
    fallback: tokio::time::Interval,
}

impl<T: Trigger> HybridTrigger<T> {
    pub fn new(event: T, period: Duration) -> Self {
        Self {
            event,
            fallback: tokio::time::interval(period),
        }
    }
}

#[async_trait::async_trait]
impl<T: Trigger + Send> Trigger for HybridTrigger<T> {
    async fn wait(&mut self) {
        tokio::select! {
            () = self.event.wait() => {}
            _ = self.fallback.tick() => {}
        }
    }
}
