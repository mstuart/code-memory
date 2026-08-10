// File watcher for incremental index updates
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
    changed_files: HashSet<PathBuf>,
    last_event_time: Option<Instant>,
    debounce_duration: Duration,
}

impl FileWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> notify::Result<Self> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )?;

        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            changed_files: HashSet::new(),
            last_event_time: None,
            debounce_duration: Duration::from_millis(500),
        })
    }

    pub fn get_changes(&mut self) -> Vec<PathBuf> {
        let now = Instant::now();

        // Process all pending events and record the time we saw the last one
        let mut saw_events = false;
        while let Ok(Ok(event)) = self.receiver.try_recv() {
            saw_events = true;
            match event.kind {
                notify::EventKind::Modify(_) | notify::EventKind::Create(_) => {
                    for path in event.paths {
                        // Track file paths (for Create events, file might not exist yet if it's being written)
                        // We use extension to determine if it's likely a file
                        if path.is_file() || path.extension().is_some() {
                            self.changed_files.insert(path);
                        }
                    }
                }
                notify::EventKind::Remove(_) => {
                    for path in event.paths {
                        // For deletions, we can't check is_file(), so just add all paths
                        self.changed_files.insert(path);
                    }
                }
                _ => {}
            }
        }

        // Update last event time if we saw any events
        if saw_events {
            self.last_event_time = Some(now);
        }

        // Return changes if we have any and no events in the last debounce period
        if !self.changed_files.is_empty() {
            if let Some(last) = self.last_event_time {
                if last.elapsed() >= self.debounce_duration {
                    let changes: Vec<PathBuf> = self.changed_files.drain().collect();
                    self.last_event_time = None;
                    return changes;
                }
            }
        }

        vec![]
    }

    pub fn has_changes(&mut self) -> bool {
        !self.get_changes().is_empty()
    }
}
