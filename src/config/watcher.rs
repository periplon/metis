use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use tracing::{error, info};

pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    pub fn new<F>(paths: Vec<String>, on_change: F) -> Result<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let (tx, rx) = channel();

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

        // Add paths to be watched.
        for path in &paths {
            if Path::new(path).exists() {
                watcher.watch(Path::new(path), RecursiveMode::Recursive)?;
                info!("Watching configuration path: {}", path);
            } else {
                tracing::warn!("Configuration path does not exist, skipping: {}", path);
            }
        }

        // Spawn a thread to handle events
        std::thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(Ok(_event)) => {
                        // Debounce slightly by waiting
                        std::thread::sleep(Duration::from_millis(100));
                        info!("Configuration change detected, reloading...");
                        on_change();
                    }
                    Ok(Err(e)) => error!("Watch error: {:?}", e),
                    Err(e) => {
                        error!("Watch channel error: {:?}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self { watcher })
    }
}
