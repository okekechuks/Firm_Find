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
    Refresh(u64, mpsc::Sender<DiscoverySnapshot>),
}

pub struct DiscoveryService {
    snapshot: Arc<Mutex<DiscoverySnapshot>>,
    sender: SyncSender<Signal>,
    stopped: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
    generation: Arc<Mutex<u64>>,
}

impl DiscoveryService {
    pub fn start(
        app: tauri::AppHandle,
        repository: Arc<Mutex<Repository>>,
    ) -> std::io::Result<Self> {
        let snapshot = Arc::new(Mutex::new(DiscoverySnapshot::default()));
        let (sender, receiver) = mpsc::sync_channel(64);
        let stopped = Arc::new(AtomicBool::new(false));
        let generation = Arc::new(Mutex::new(0));
        let epoch = Arc::clone(&generation);
        let state = Arc::clone(&snapshot);
        let stop = Arc::clone(&stopped);
        let notifications_sender = sender.clone();
        let worker = thread::Builder::new()
            .name("obtainer-discovery".into())
            .spawn(move || {
                run_worker(
                    app,
                    repository,
                    state,
                    (receiver, notifications_sender),
                    stop,
                    epoch,
                );
            })?;
        Ok(Self {
            snapshot,
            sender,
            stopped,
            worker: Mutex::new(Some(worker)),
            generation,
        })
    }

    pub(crate) fn shared_snapshot(&self) -> Arc<Mutex<DiscoverySnapshot>> {
        Arc::clone(&self.snapshot)
    }

    pub fn snapshot(&self) -> Result<DiscoverySnapshot, &'static str> {
        self.snapshot
            .lock()
            .map(|value| value.clone())
            .map_err(|_| "Discovery state is unavailable. Restart OBTAINER.")
    }

    pub fn refresh_request(&self) -> Result<Receiver<DiscoverySnapshot>, &'static str> {
        // Commit admission and generation together. Rejected requests cannot
        // invalidate an already admitted scan.
        let mut generation = self
            .generation
            .lock()
            .map_err(|_| "Discovery generation unavailable")?;
        let epoch = *generation + 1;
        let (reply, receiver) = mpsc::channel();
        self.sender
            .try_send(Signal::Refresh(epoch, reply))
            .map_err(|_| "Discovery is busy. Wait a moment and refresh again.")?;
        *generation = epoch;
        Ok(receiver)
    }

    pub fn cancel(&self) {
        if let Ok(mut generation) = self.generation.lock() {
            *generation += 1;
        }
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
    channel: (Receiver<Signal>, SyncSender<Signal>),
    stopped: Arc<AtomicBool>,
    generation: Arc<Mutex<u64>>,
) {
    let (receiver, sender) = channel;
    // Register before initial enumeration so arrivals during startup cannot be missed.
    #[cfg(windows)]
    let notifications = super::windows::Notifications::register(sender);
    #[cfg(not(windows))]
    let _ = sender;
    let mut snapshot = DiscoverySnapshot::default();
    if let Ok(repository) = repository.lock() {
        match repository.discovery_history() {
            Ok(devices) => snapshot.devices = devices,
            Err(error) => tracing::warn!(%error, "Device summaries could not be loaded"),
        }
    }
    #[cfg(windows)]
    {
        snapshot.watching = notifications.watching();
        snapshot.warnings.clone_from(&notifications.warnings);
    }
    scan_and_reply(
        &repository,
        &state,
        &app,
        &mut snapshot,
        vec![],
        &generation,
    );
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
        if let Some(Signal::Refresh(epoch, reply)) = first {
            replies.push((epoch, reply));
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        // Debounce event bursts, but cap delay so repeated PnP traffic cannot starve refreshes.
        while Instant::now() < deadline && !stopped.load(Ordering::Acquire) {
            match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(Signal::Refresh(epoch, reply)) => replies.push((epoch, reply)),
                Ok(Signal::Changed) => {}
                Err(_) => break,
            }
        }
        if stopped.load(Ordering::Acquire) {
            break;
        }
        let automatic = repository
            .lock()
            .ok()
            .and_then(|r| r.desktop_preferences().ok())
            .map(|p| p.auto_refresh)
            .unwrap_or(true);
        if replies.is_empty() && !automatic {
            continue;
        }
        // Replies are fulfilled after the following scan, not with an earlier inventory.
        scan_and_reply(
            &repository,
            &state,
            &app,
            &mut snapshot,
            replies,
            &generation,
        );
    }
}

fn scan_and_reply(
    repository: &Arc<Mutex<Repository>>,
    state: &Arc<Mutex<DiscoverySnapshot>>,
    app: &tauri::AppHandle,
    snapshot: &mut DiscoverySnapshot,
    replies: Vec<(u64, mpsc::Sender<DiscoverySnapshot>)>,
    generation: &Mutex<u64>,
) {
    let epoch = *generation.lock().expect("Discovery generation poisoned");
    let queued_cancelled = !replies.is_empty() && replies.iter().all(|(run, _)| *run != epoch);
    let started = Instant::now();
    #[cfg(windows)]
    let capture = if queued_cancelled {
        Err("cancelled".into())
    } else {
        super::windows::capture()
    };
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
    let commit = generation.lock().expect("Discovery generation poisoned");
    if epoch != *commit || queued_cancelled {
        mark_cancelled(snapshot);
    } else {
        snapshot.cancelled = false;
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
    }
    snapshot.duration_ms = started.elapsed().as_millis() as u64;
    if let Ok(mut current) = state.lock() {
        *current = snapshot.clone();
    }
    use tauri::Manager;
    if let Some(adapters) = app.try_state::<crate::adapters::SessionManager>() {
        adapters.observe_discovery(snapshot);
    }
    if let Some(openwrt) = app.try_state::<Arc<crate::openwrt::runtime::Service>>() {
        openwrt.observe_discovery(snapshot);
    }
    // Events contain a revision only, never hardware identifiers.
    let _ = app.emit("discovery-changed", snapshot.revision);
    for (run, reply) in replies {
        let mut response = snapshot.clone();
        if run != *commit {
            response.cancelled = true;
        }
        let _ = reply.send(response);
    }
}

fn mark_failed(snapshot: &mut DiscoverySnapshot, warning: &str, detail: &str) {
    tracing::error!(detail, "Device inventory refresh failed");
    snapshot.status = DiscoveryStatus::Failed;
    snapshot.warnings.push(warning.into());
}

fn mark_cancelled(snapshot: &mut DiscoverySnapshot) {
    // No inventory was committed. Existing evidence retains its original age.
    snapshot.cancelled = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_or_failed_scan_retains_authority_revision_and_age() {
        let mut snapshot = DiscoverySnapshot {
            revision: 41,
            scanned_at: Some(12),
            ..DiscoverySnapshot::default()
        };
        mark_cancelled(&mut snapshot);
        assert_eq!(snapshot.revision, 41);
        assert_eq!(snapshot.scanned_at, Some(12));
        assert!(snapshot.cancelled);
        mark_failed(&mut snapshot, "retained", "test capture failure");
        assert_eq!(snapshot.revision, 41);
        assert_eq!(snapshot.scanned_at, Some(12));
        assert!(matches!(snapshot.status, DiscoveryStatus::Failed));
    }
    #[test]
    fn review_rejected_refresh_does_not_cancel_admitted_scan() {
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.try_send(Signal::Changed).unwrap();
        let service = DiscoveryService {
            sender,
            snapshot: Arc::new(Mutex::new(DiscoverySnapshot::default())),
            stopped: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
            generation: Arc::new(Mutex::new(42)),
        };
        assert!(service.refresh_request().is_err());
        assert_eq!(*service.generation.lock().unwrap(), 42);
        receiver.recv().unwrap();
        let _reply = service.refresh_request().unwrap();
        assert_eq!(*service.generation.lock().unwrap(), 43);
        assert!(matches!(receiver.recv().unwrap(), Signal::Refresh(43, _)));
    }
}
