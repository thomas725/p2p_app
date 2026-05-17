use super::constants::FRAME_TIME_MS;
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
            if poll(Duration::from_millis(FRAME_TIME_MS)).ok() == Some(true)
                && let Ok(event) = read()
                && let Some(input) = crossterm_event_to_input_event(event)
            {
                let _ = input_tx.send(input).await;
            }
            tokio::task::yield_now().await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    #[test]
    fn test_key_event_maps_to_input_event() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let event = Event::Key(key);
        let result = crossterm_event_to_input_event(event);
        assert!(matches!(result, Some(InputEvent::Key(k)) if k.code == KeyCode::Char('a')));
    }

    #[test]
    fn test_mouse_event_maps_to_input_event() {
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 3,
            modifiers: KeyModifiers::NONE,
        };
        let event = Event::Mouse(mouse);
        let result = crossterm_event_to_input_event(event);
        assert!(matches!(result, Some(InputEvent::Mouse(m)) if m.column == 5 && m.row == 3));
    }

    #[test]
    fn test_resize_event_returns_none() {
        let event = Event::Resize(80, 24);
        let result = crossterm_event_to_input_event(event);
        assert!(result.is_none());
    }

    #[test]
    fn test_focus_event_returns_none() {
        let event = Event::FocusGained;
        let result = crossterm_event_to_input_event(event);
        assert!(result.is_none());
    }

    #[test]
    fn test_paste_event_returns_none() {
        let event = Event::Paste("hello".to_string());
        let result = crossterm_event_to_input_event(event);
        assert!(result.is_none());
    }
}
