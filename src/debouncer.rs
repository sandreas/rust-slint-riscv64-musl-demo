use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time;

pub struct AsyncDebouncer {
    state: Arc<Mutex<DebounceState>>,
    current_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    delay: Duration,
}

#[derive(Default)]
struct DebounceState {
    is_hold: bool,
}

impl AsyncDebouncer {
    pub fn new(delay: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(DebounceState::default())),
            current_task: Arc::new(Mutex::new(None)),
            delay,
        }
    }

    pub fn trigger<F>(
        &self,
        clicks: Arc<Mutex<i32>>,
        is_hold: bool,
        main_callback: F,
    )
    where
        F: Fn(Arc<Mutex<i32>>, bool) + Send + 'static,
    {
        // ✅ 1. Update state FIRST
        {
            let mut s = self.state.lock().unwrap();
            s.is_hold = is_hold;
        }

        // ✅ 2. Abort PREVIOUS task (not the current one)
        let prev_task = {
            let mut tasks = self.current_task.lock().unwrap();
            tasks.take()
        };
        drop(prev_task); // Abort previous

        // ✅ 3. Clone everything for the NEW task
        let state = Arc::clone(&self.state);
        let clicks_task = clicks.clone();
        let current_task = Arc::clone(&self.current_task);
        let delay = self.delay;

        // ✅ 4. Spawn NEW task
        let handle = tokio::spawn(async move {
            time::sleep(delay).await;

            // ✅ 5. Execute callback - this survives!
            let s = state.lock().unwrap();
            main_callback(clicks_task, s.is_hold);
        });

        // ✅ 6. Store NEW handle for next trigger to cancel
        let mut tasks = self.current_task.lock().unwrap();
        *tasks = Some(handle);
    }
}

