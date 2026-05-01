//! Installs and removes the signature bypass files that allow unsigned pak mods to load.

use std::fs;
use std::path::PathBuf;

use crate::paths::{binaries_dir, mods_dir};

use super::{BYPASS_ASI, BYPASS_DSOUND, file_matches};

struct BypassPaths {
    dsound: PathBuf,
    asi: PathBuf,
}

fn bypass_paths(game_root: &str) -> BypassPaths {
    let bin_dir = binaries_dir(game_root);
    BypassPaths {
        dsound: bin_dir.join("dsound.dll"),
        asi: bin_dir
            .join("plugins")
            .join("MarvelRivalsUTOCSignatureBypass.asi"),
    }
}

/// dsound.dll is a generic ASI loader; only the .asi payload must byte-match.
pub(crate) fn is_signature_bypass_installed(game_root: &str) -> bool {
    let paths = bypass_paths(game_root);
    paths.dsound.exists() && file_matches(&paths.asi, BYPASS_ASI)
}

pub(crate) fn install_signature_bypass(game_root: &str) -> Result<String, String> {
    // Validate that the bundled DLL is a real PE binary (MZ header), not a placeholder.
    if !BYPASS_DSOUND.starts_with(b"MZ") {
        return Err("Bundled dsound.dll is a placeholder. \
             Replace src-tauri/resources/bypass/dsound.dll with the real file \
             from the Nexusmods bypass mod and rebuild the app."
            .to_string());
    }

    let bin_dir = binaries_dir(game_root);
    if !bin_dir.exists() {
        return Err(format!(
            "Binaries directory not found: {}\nMake sure the game root path is correct.",
            bin_dir.display()
        ));
    }

    let paths = bypass_paths(game_root);
    // Don't overwrite a user-supplied loader; any dsound.dll counts as present.
    let dsound_ok = paths.dsound.exists();
    let asi_ok = file_matches(&paths.asi, BYPASS_ASI);
    let mods_ok = mods_dir(game_root).exists();

    if dsound_ok && asi_ok && mods_ok {
        return Ok("Signature bypass is already installed and up to date.".to_string());
    }

    if !dsound_ok {
        fs::write(&paths.dsound, BYPASS_DSOUND).map_err(|e| e.to_string())?;
    }

    if !asi_ok {
        if let Some(plugins_dir) = paths.asi.parent() {
            fs::create_dir_all(plugins_dir).map_err(|e| e.to_string())?;
        }
        fs::write(&paths.asi, BYPASS_ASI).map_err(|e| e.to_string())?;
    }

    if !mods_ok {
        fs::create_dir_all(mods_dir(game_root)).map_err(|e| e.to_string())?;
    }

    Ok("Bypass installed successfully!".to_string())
}

pub(crate) fn remove_signature_bypass(game_root: &str) -> Result<String, String> {
    let paths = bypass_paths(game_root);

    let mut removed = 0usize;
    for path in &[&paths.dsound, &paths.asi] {
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
