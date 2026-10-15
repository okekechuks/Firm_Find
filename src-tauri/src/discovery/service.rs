use super::{DiscoverySnapshot, DiscoveryStatus};
use crate::storage::Repository;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Emitter;

pub(super) enum Signal {
    Changed,
    Refresh(mpsc::Sender<DiscoverySnapshot>),
}

pub struct DiscoveryService {
    snapshot: Arc<Mutex<DiscoverySnapshot>>,
    sender: SyncSender<Signal>,
    stopped: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl DiscoveryService {
    pub fn start(
        app: tauri::AppHandle,
        repository: Arc<Mutex<Repository>>,
    ) -> std::io::Result<Self> {
        let snapshot = Arc::new(Mutex::new(DiscoverySnapshot::default()));
        let (sender, receiver) = mpsc::sync_channel(64);
        let stopped = Arc::new(AtomicBool::new(false));
        let state = Arc::clone(&snapshot);
        let stop = Arc::clone(&stopped);
        let notifications_sender = sender.clone();
        let worker = thread::Builder::new()
            .name("obtainer-discovery".into())
            .spawn(move || {
                run_worker(app, repository, state, receiver, notifications_sender, stop);
            })?;
        Ok(Self {
            snapshot,
            sender,
            stopped,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn snapshot(&self) -> Result<DiscoverySnapshot, &'static str> {
        self.snapshot
            .lock()
            .map(|value| value.clone())
            .map_err(|_| "Discovery state is unavailable. Restart OBTAINER.")
    }

    pub fn refresh_request(&self) -> Result<Receiver<DiscoverySnapshot>, &'static str> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .try_send(Signal::Refresh(reply))
            .map_err(|_| "Discovery is busy. Wait a moment and refresh again.")?;
        Ok(receiver)
    }

    pub fn shutdown(&self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.sender.try_send(Signal::Changed);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for DiscoveryService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_worker(
    app: tauri::AppHandle,
    repository: Arc<Mutex<Repository>>,
    state: Arc<Mutex<DiscoverySnapshot>>,
    receiver: Receiver<Signal>,
    sender: SyncSender<Signal>,
    stopped: Arc<AtomicBool>,
) {
    // Register before initial enumeration so arrivals during startup cannot be missed.
    #[cfg(windows)]
    let notifications = super::windows::Notifications::register(sender);
    #[cfg(not(windows))]
    let _ = sender;
    let mut snapshot = DiscoverySnapshot::default();
    #[cfg(windows)]
    {
        snapshot.watching = notifications.watching();
        snapshot.warnings.clone_from(&notifications.warnings);
    }
    scan_and_reply(&repository, &state, &app, &mut snapshot, vec![]);
    loop {
        if stopped.load(Ordering::Acquire) {
            break;
        }
        let first = match receiver.recv_timeout(Duration::from_secs(30)) {
            Ok(signal) => Some(signal),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let mut replies = vec![];
        if let Some(Signal::Refresh(reply)) = first {
            replies.push(reply);
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        // Debounce event bursts, but cap delay so repeated PnP traffic cannot starve refreshes.
        while Instant::now() < deadline && !stopped.load(Ordering::Acquire) {
            match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(Signal::Refresh(reply)) => replies.push(reply),
                Ok(Signal::Changed) => {}
                Err(_) => break,
            }
        }
        if stopped.load(Ordering::Acquire) {
            break;
        }
        // Replies are fulfilled after the following scan, not with an earlier inventory.
        scan_and_reply(&repository, &state, &app, &mut snapshot, replies);
    }
}

fn scan_and_reply(
    repository: &Arc<Mutex<Repository>>,
    state: &Arc<Mutex<DiscoverySnapshot>>,
    app: &tauri::AppHandle,
    snapshot: &mut DiscoverySnapshot,
    replies: Vec<mpsc::Sender<DiscoverySnapshot>>,
) {
    #[cfg(windows)]
    let capture = super::windows::capture();
    #[cfg(not(windows))]
    let capture: Result<super::Inventory, String> =
        Err("Native discovery is available only on Windows.".into());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    snapshot
        .warnings
        .retain(|warning| warning.contains("notifications unavailable"));
    match capture {
        Ok(inventory) => {
            let mut next = snapshot.clone(); next.reconcile(inventory, now);
            let saved = repository.lock().map_err(|_| "Database lock failed".to_owned())
                .and_then(|mut repository| repository.save_discovery(&next).map_err(|error| error.to_string()));
            match saved {
                Ok(()) => *snapshot = next,
                Err(error) => mark_failed(snapshot, "Could not save inventory. Last successful inventory is retained.", &error),
            }
        }
        Err(error) => mark_failed(snapshot, "Windows inventory could not be refreshed. Last successful inventory is retained; try Refresh.", &error),
    }
    if let Ok(mut current) = state.lock() {
        *current = snapshot.clone();
    }
    // Events contain a revision only, never hardware identifiers.
    let _ = app.emit("discovery-changed", snapshot.revision);
    for reply in replies {
        let _ = reply.send(snapshot.clone());
    }
}

fn mark_failed(snapshot: &mut DiscoverySnapshot, warning: &str, detail: &str) {
    tracing::error!(detail, "Device inventory refresh failed");
    snapshot.status = DiscoveryStatus::Failed;
    snapshot.revision += 1;
    snapshot.warnings.push(warning.into());
}
