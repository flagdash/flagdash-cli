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
                match event::poll(tick_rate) {
                    Ok(true) => match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if event_tx.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                        // clippy suggests folding this into a match guard. Declined
                        // on purpose: the condition sends on the channel, and a
                        // guard that mutates state while deciding whether an arm
                        // matches is far harder to read than the nested `if`.
                        #[allow(clippy::collapsible_match)]
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if event_tx.send(Event::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    },
                    // No event within tick_rate: poll already waited, so emitting
                    // a Tick here is correctly paced (~1 per tick_rate).
                    Ok(false) => {
                        if event_tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    // poll errored (e.g. stdin closed). Previously this fell into
                    // the `else` branch and emitted a Tick immediately with no
                    // delay, spinning the loop at 100% CPU. Back off for a full
                    // tick before retrying so a persistent error can't busy-loop.
                    Err(_) => {
                        tokio::time::sleep(tick_rate).await;
                        if event_tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
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
