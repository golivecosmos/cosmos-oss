use crate::{app_log_info, app_log_warn};

/// Probe whether the process has macOS Full Disk Access. Attempts to read a
/// known FDA-gated directory (`~/Library/Safari`, present on every macOS
/// install and always TCC-protected). `Ok(true)` means we can read it,
/// `Ok(false)` means access is denied, `Err` means the check could not be
/// performed (e.g. HOME unset, directory missing on an unusual system).
///
/// Off macOS this function returns `Ok(true)` unconditionally — there is no
/// equivalent permission model to gate on, so the UI banner stays hidden.
#[cfg(target_os = "macos")]
pub fn has_full_disk_access() -> anyhow::Result<bool> {
    let home = std::env::var("HOME").map_err(|e| anyhow::anyhow!("HOME not set: {}", e))?;
    let probe_path = std::path::PathBuf::from(home).join("Library/Safari");

    if !probe_path.exists() {
        // Nothing sensible to probe against; treat as "no signal" and let the
        // caller proceed. Users on heavily customized systems will indexing-
        // error instead of seeing the banner.
        app_log_warn!(
            "FDA probe: {} missing, cannot determine Full Disk Access",
            probe_path.display()
        );
        return Err(anyhow::anyhow!("FDA probe path missing"));
    }

    match std::fs::read_dir(&probe_path) {
        Ok(_) => {
            app_log_info!("FDA probe: Full Disk Access granted");
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            app_log_info!("FDA probe: Full Disk Access denied");
            Ok(false)
        }
        Err(e) => Err(anyhow::anyhow!("FDA probe failed: {}", e)),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn has_full_disk_access() -> anyhow::Result<bool> {
    Ok(true)
}
