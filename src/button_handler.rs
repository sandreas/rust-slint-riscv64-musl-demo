use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    Press,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum ButtonKey {
    PlayPause,
    VolumeUp,
    VolumeDown,
}

const DEBOUNCE_DELAY: Duration = Duration::from_millis(400);

#[derive(Clone)]
pub struct ButtonHandler {
    state: Arc<Mutex<State>>,
}

#[derive(Debug)]
struct State {
    is_down: bool,
    clicks: u32,
    timer_start: Instant,
    fired: bool,
}

impl ButtonHandler {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                is_down: false,
                clicks: 0,
                timer_start: Instant::now(),
                fired: false,
            })),
        }
    }

    pub fn on_click(&self, f: impl FnOnce() + Send + 'static) {
        let state = Arc::clone(&self.state);
        thread::spawn(move || {
            thread::sleep(DEBOUNCE_DELAY);
            let s = state.lock().unwrap();
            if !s.is_down && s.clicks > 0 && !s.fired {
                drop(s);
                f();
            }
        });
    }

    pub fn on_hold(&self, f: impl FnOnce() + Send + 'static) {
        let state = Arc::clone(&self.state);
        thread::spawn(move || {
            thread::sleep(DEBOUNCE_DELAY);
            let s = state.lock().unwrap();
            if s.is_down && !s.fired {
                drop(s);
                f();
            }
        });
    }

    /// Single entry point
    pub fn handle_button_event(&self, action: ButtonAction) {
        let mut s = self.state.lock().unwrap();
        s.timer_start = Instant::now(); // Reset timer
        s.fired = false;

        match action {
            ButtonAction::Press => {
                if !s.is_down {
                    s.is_down = true;
                    s.clicks += 1;
                }
            }
            ButtonAction::Release => s.is_down = false,
        }
    }
}