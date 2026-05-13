//! Installs and removes the signature bypass that allows unsigned pak mods to load. Detects either the legacy dsound.dll + ASI loader or the modern version.dll proxy.

use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use crate::paths::{binaries_dir, mods_dir};

use super::BYPASS_VERSION_DLL;

const VERSION_DLL_FILENAME: &str = "version.dll";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BypassKind {
    None,
    Legacy,
    Modern,
}

struct BypassPaths {
    dsound: PathBuf,
    asi: PathBuf,
    version_dll: PathBuf,
}

fn bypass_paths(game_root: &str) -> BypassPaths {
    let bin_dir = binaries_dir(game_root);
    BypassPaths {
        dsound: bin_dir.join("dsound.dll"),
        asi: bin_dir
            .join("plugins")
            .join("MarvelRivalsUTOCSignatureBypass.asi"),
        version_dll: bin_dir.join(VERSION_DLL_FILENAME),
    }
}

/// A `version.dll` in the game's exe directory is always a DLL-proxy hijack,
/// so its presence alone marks the modern bypass. Legacy detection pairs a
/// generic `dsound.dll` with the specifically-named ASI loader so a stray
/// third-party `dsound.dll` doesn't get mistaken for ours.
pub(crate) fn bypass_install_kind(game_root: &str) -> BypassKind {
    let paths = bypass_paths(game_root);
    if paths.version_dll.exists() {
        return BypassKind::Modern;
    }
    if paths.dsound.exists() && paths.asi.exists() {
        return BypassKind::Legacy;
    }
    BypassKind::None
}

pub(crate) fn is_signature_bypass_installed(game_root: &str) -> bool {
    bypass_install_kind(game_root) != BypassKind::None
}

pub(crate) fn install_signature_bypass(game_root: &str) -> Result<String, String> {
    if !BYPASS_VERSION_DLL.starts_with(b"MZ") {
        return Err("Bundled version.dll is a placeholder. \
             Build oZanderr/rivals-sigbypass (proxy branch) and copy the produced \
             version.dll into src-tauri/resources/bypass/, then rebuild the app."
            .to_string());
    }

    let bin_dir = binaries_dir(game_root);
    if !bin_dir.exists() {
        return Err(format!(
            "Binaries directory not found: {}\nMake sure the game root path is correct.",
            bin_dir.display()
        ));
    }

    match bypass_install_kind(game_root) {
        BypassKind::Modern => {
            return Ok("Signature bypass already installed (version.dll).".to_string());
        }
        BypassKind::Legacy => {
            return Err(
                "Legacy bypass is installed. Remove it first to install the new \
                 version.dll proxy."
                    .to_string(),
            );
        }
        BypassKind::None => {}
    }

    let paths = bypass_paths(game_root);
    fs::write(&paths.version_dll, BYPASS_VERSION_DLL)
        .map_err(|e| format!("write version.dll: {e}"))?;

    if !mods_dir(game_root).exists() {
        fs::create_dir_all(mods_dir(game_root)).map_err(|e| e.to_string())?;
    }

    Ok("Bypass installed successfully!".to_string())
}

pub(crate) fn remove_signature_bypass(game_root: &str) -> Result<String, String> {
    let paths = bypass_paths(game_root);

    let mut removed = 0usize;
    for path in &[&paths.version_dll, &paths.dsound, &paths.asi] {
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
            removed += 1;
        }
    }

    if removed == 0 {
        Ok("Bypass files were not present!".to_string())
    } else {
        Ok(format!("Removed {removed} bypass file(s)"))
    }
}
