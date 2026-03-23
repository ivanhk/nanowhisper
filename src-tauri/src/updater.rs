use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

static CHECKING: AtomicBool = AtomicBool::new(false);

pub struct PendingUpdate {
    pub version: String,
    pub update: tauri_plugin_updater::Update,
    pub bytes: Vec<u8>,
}

pub struct UpdateState {
    pub pending: Mutex<Option<PendingUpdate>>,
}

pub fn init(app_handle: &AppHandle) {
    // Always register state so IPC commands don't panic
    app_handle.manage(UpdateState {
        pending: Mutex::new(None),
    });

    // Skip background update checks in development
    if cfg!(debug_assertions) {
        log::info!("Updater: skipped (debug build)");
        return;
    }

    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        // Wait 10 seconds after startup before first check
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        loop {
            match check_and_download(&handle).await {
                Ok(()) => {}
                Err(e) => log::warn!("Update check failed: {}", e),
            }
            // Check every 4 hours
            tokio::time::sleep(std::time::Duration::from_secs(4 * 60 * 60)).await;
        }
    });

    log::info!("Updater: initialized");
}

pub async fn check_and_download(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Prevent concurrent checks (background loop vs manual trigger)
    if CHECKING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::info!("Updater: check already in progress, skipping");
        return Ok(());
    }

    let result = do_check_and_download(app).await;
    CHECKING.store(false, Ordering::SeqCst);
    result
}

async fn do_check_and_download(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Updater: checking for updates...");

    let update = app.updater()?.check().await?;

    if let Some(update) = update {
        let version = update.version.clone();
        log::info!("Updater: new version available: {}", version);

        // Skip if we already downloaded this version
        {
            let state = app.state::<UpdateState>();
            let pending = state.pending.lock().unwrap();
            if pending.as_ref().is_some_and(|p| p.version == version) {
                log::info!("Updater: v{} already downloaded, skipping", version);
                return Ok(());
            }
        }

        let _ = app.emit(
            "update-available",
            serde_json::json!({ "version": &version }),
        );

        // Download silently in background
        let bytes = update
            .download(
                |chunk_len, content_len| {
                    log::debug!(
                        "Updater: downloaded {} / {:?} bytes",
                        chunk_len,
                        content_len
                    );
                },
                || {
                    log::info!("Updater: download complete");
                },
            )
            .await?;

        // Store the downloaded update for later installation
        let state = app.state::<UpdateState>();
        state.pending.lock().unwrap().replace(PendingUpdate {
            version: version.clone(),
            update,
            bytes,
        });

        let _ = app.emit(
            "update-downloaded",
            serde_json::json!({ "version": &version }),
        );
        log::info!("Updater: v{} ready to install", version);
    } else {
        log::info!("Updater: already up to date");
    }

    Ok(())
}
