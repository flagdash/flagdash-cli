use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Application events: either a terminal event or a periodic tick.
#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
}

/// Polls crossterm events and sends them through an mpsc channel.
/// Runs on a dedicated tokio task.
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate in milliseconds.
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();
        let tick_rate = Duration::from_millis(tick_rate_ms);

        tokio::spawn(async move {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if event_tx.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if event_tx.send(Event::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                } else if event_tx.send(Event::Tick).is_err() {
                    break;
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Receive the next event, blocking until one is available.
    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("event channel closed"))
    }
}
