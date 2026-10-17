//! One worker with a replaceable pending slot. Stale work never reaches the UI.
use crate::i18n::Key;
use crate::{
    import::{self, ImportError, Limits},
    scene::Scene,
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
};

struct Request {
    id: u64,
    path: PathBuf,
    cancel: Arc<AtomicBool>,
    limits: Limits,
}
pub struct Loaded {
    pub id: u64,
    pub path: PathBuf,
    pub result: Result<crate::prepared::Prepared, ImportError>,
}
#[derive(Default)]
struct State {
    pending: Option<Request>,
    finished: Option<Loaded>,
    shutdown: bool,
}
pub struct Loader {
    shared: Arc<(Mutex<State>, Condvar)>,
    cancel: Arc<AtomicBool>,
    pub current: u64,
    pub loading: bool,
    progress: Arc<AtomicU8>,
}
impl Loader {
    pub fn new(wake: impl Fn() + Send + 'static) -> std::io::Result<Self> {
        Self::with_import(wake, import::load_with_progress)
    }
    fn with_import(
        wake: impl Fn() + Send + 'static,
        import: impl Fn(&std::path::Path, &Limits, &AtomicBool, &AtomicU8) -> Result<Scene, ImportError>
        + Send
        + 'static,
    ) -> std::io::Result<Self> {
        let shared = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let worker = shared.clone();
        let progress = Arc::new(AtomicU8::new(0));
        let worker_progress = progress.clone();
        thread::Builder::new()
            .name("3mf-import".into())
            .spawn(move || {
                loop {
                    let request = {
                        let (lock, cv) = &*worker;
                        let Ok(mut state) = lock.lock() else { return };
                        while state.pending.is_none() && !state.shutdown {
                            state = match cv.wait(state) {
                                Ok(s) => s,
                                Err(_) => return,
                            };
                        }
                        if state.shutdown {
                            return;
                        }
                        match state.pending.take() {
                            Some(r) => r,
                            None => continue,
                        }
                    };
                    let result = import(
                        &request.path,
                        &request.limits,
                        &request.cancel,
                        &worker_progress,
                    )
                    .and_then(|scene| {
                        crate::prepared::prepare(scene, &request.limits, &request.cancel)
                    });
                    if let Ok(mut state) = worker.0.lock() {
                        // Check under the same lock as cancel/open: a stale result must
                        // not retain a second large scene while the next import runs.
                        if !state.shutdown && !request.cancel.load(Ordering::Acquire) {
                            state.finished = Some(Loaded {
                                id: request.id,
                                path: request.path,
                                result,
                            });
                            drop(state);
                            wake();
                        }
                    }
                }
            })?;
        Ok(Self {
            shared,
            cancel: Arc::new(AtomicBool::new(false)),
            current: 0,
            loading: false,
            progress,
        })
    }
    pub fn stage(&self) -> Key {
        match self.progress.load(Ordering::Relaxed) {
            1 => Key::ReadArchive,
            2 => Key::ParseModel,
            3 => Key::PrepareScene,
            _ => Key::WaitLoad,
        }
    }
    pub fn open(&mut self, path: PathBuf, resident_bytes: usize) {
        self.cancel();
        self.progress.store(0, Ordering::Relaxed);
        self.current = self.current.wrapping_add(1);
        self.cancel = Arc::new(AtomicBool::new(false));
        let mut limits = Limits::default();
        limits.memory_bytes = limits.memory_bytes.saturating_sub(resident_bytes);
        if let Ok(mut state) = self.shared.0.lock() {
            state.finished = None;
            state.pending = Some(Request {
                id: self.current,
                path,
                cancel: self.cancel.clone(),
                limits,
            });
            self.loading = true;
        }
        self.shared.1.notify_one();
    }
    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.loading = false;
        if let Ok(mut state) = self.shared.0.lock() {
            state.pending = None;
            state.finished = None;
        }
    }
    pub fn poll(&mut self) -> Option<Loaded> {
        let result = self.shared.0.lock().ok()?.finished.take()?;
        if result.id != self.current || self.cancel.load(Ordering::Acquire) {
            return None;
        }
        self.loading = false;
        Some(result)
    }
}
impl Drop for Loader {
    fn drop(&mut self) {
        self.cancel();
        if let Ok(mut s) = self.shared.0.lock() {
            s.shutdown = true;
        }
        self.shared.1.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};
    #[test]
    fn latest_request_replaces_pending_work_and_cancel_discards_result() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (wake_tx, wake_rx) = mpsc::channel();
        let mut loader = Loader::with_import(
            move || {
                let _ = wake_tx.send(());
            },
            move |path, _, cancel, _| {
                entered_tx.send(path.to_path_buf()).unwrap();
                if path == std::path::Path::new("a") {
                    release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                }
                if cancel.load(Ordering::Acquire) {
                    Err(ImportError::Cancelled)
                } else {
                    Err(ImportError::Empty)
                }
            },
        )
        .unwrap();
        loader.open("a".into(), 0);
        assert_eq!(
            entered_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
            PathBuf::from("a")
        );
        loader.open("b".into(), 0);
        loader.open("c".into(), 0);
        release_tx.send(()).unwrap();
        assert_eq!(
            entered_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
            PathBuf::from("c")
        );
        wake_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(loader.poll().unwrap().path, PathBuf::from("c"));
        assert!(!loader.loading);
        loader.cancel();
        assert!(loader.poll().is_none());
    }
}
