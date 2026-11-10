use std::cell::Cell;

/// Marks the thread that draws the world, so the entity light lookup is raised there only. The server thread (mob
/// spawning, in single player too) and the terrain mesh workers keep the real light. The flag is set once by the
/// `client_update` detour (update and world drawing share that thread) and never cleared.
pub struct DrawThread;

thread_local! {
    static DRAWS: Cell<bool> = const { Cell::new(false) };
}

impl DrawThread {
    /// Marks the calling thread as the draw thread.
    pub fn mark() {
        DRAWS.with(|flag| flag.set(true));
    }

    /// True on the marked thread. One thread-local read; called on every lookup, on every thread.
    pub fn is_current() -> bool {
        DRAWS.with(Cell::get)
    }
}

#[cfg(test)]
mod tests {
    use super::DrawThread;

    #[test]
    fn fresh_thread_is_not_draw_thread() {
        let seen = std::thread::spawn(DrawThread::is_current).join();
        assert!(matches!(seen, Ok(false)));
    }

    #[test]
    fn mark_sets_only_this_thread() {
        let marked = std::thread::spawn(|| {
            DrawThread::mark();
            let here = DrawThread::is_current();
            let other = std::thread::spawn(DrawThread::is_current).join();
            (here, other.ok())
        })
        .join();
        assert!(matches!(marked, Ok((true, Some(false)))));
    }
}
