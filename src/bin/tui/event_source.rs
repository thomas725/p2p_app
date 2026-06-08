use super::state::FRAME_TIME_MS;
use crossterm::event::{Event, KeyEvent, MouseEvent, poll, read};
use std::time::Duration;
use tokio::sync::mpsc;

/// Input event type for terminal I/O
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

/// Pure: convert crossterm `Event` to `InputEvent`
pub fn crossterm_event_to_input_event(event: Event) -> Option<InputEvent> {
    match event {
        Event::Key(key) => Some(InputEvent::Key(key)),
        Event::Mouse(mouse) => Some(InputEvent::Mouse(mouse)),
        _ => None,
    }
}

/// Spawns the input handler task that polls terminal events
pub fn spawn_input_handler(input_tx: mpsc::Sender<InputEvent>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if poll(Duration::ZERO).ok() == Some(true)
                && let Ok(event) = read()
                && let Some(input) = crossterm_event_to_input_event(event)
            {
                let _ = input_tx.send(input).await;
            } else {
                tokio::time::sleep(Duration::from_millis(FRAME_TIME_MS)).await;
            }
        }
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_event_source.rs"]
mod tests;
