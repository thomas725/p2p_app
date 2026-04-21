use crossterm::event::{poll, read, Event, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;
use super::constants::FRAME_TIME_MS;

/// Input event type for terminal I/O
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

/// Spawns the input handler task that polls terminal events
pub fn spawn_input_handler(
    input_tx: mpsc::Sender<InputEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            // Poll with FRAME_TIME_MS timeout (60 FPS)
            if poll(Duration::from_millis(FRAME_TIME_MS)).ok() == Some(true) {
                if let Ok(event) = read() {
                    match event {
                        Event::Key(key) => {
                            let _ = input_tx.send(InputEvent::Key(key)).await;
                        }
                        Event::Mouse(mouse) => {
                            let _ = input_tx.send(InputEvent::Mouse(mouse)).await;
                        }
                        _ => {}
                    }
                }
            }
            // Yield to async runtime
            tokio::task::yield_now().await;
        }
    })
}
