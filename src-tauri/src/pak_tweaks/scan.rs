//! Scans mods for pak files containing tweakable INI entries and reads their active tweak states.

use std::path::Path;

use crate::paths::mods_dir;
use crate::tweaks::TweakState;

use super::cvars::parse_console_vars;
use super::io::{extract_file_to_string, inspect_pak_for_any_ini, inspect_pak_for_ini};
use super::{PakIniInfo, PakIniListing, PakTweakState};

/// Inspect one pak and return INI metadata when present.
pub(crate) fn inspect_single_pak(pak_path: &str) -> Result<Option<PakIniInfo>, String> {
    inspect_pak_for_ini(Path::new(pak_path))
}

pub(crate) fn inspect_single_pak_any_ini(pak_path: &str) -> Result<Option<PakIniListing>, String> {
    inspect_pak_for_any_ini(Path::new(pak_path))
}

/// Scan `~mods` and return paks that contain tweakable INI files.
pub(crate) fn scan_mod_paks(game_root: &str, recursive: bool) -> Result<Vec<PakIniInfo>, String> {
    let mods_dir = mods_dir(game_root);
    if !mods_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for rel_path in crate::mods::walk_mod_files(&mods_dir, recursive) {
        let path = mods_dir.join(&rel_path);
        if path.extension().and_then(|x| x.to_str()) != Some("pak") {
            continue;
        }
        match inspect_pak_for_ini(&path) {
            Ok(Some(info)) => results.push(info),
            Ok(None) => {}
            Err(_) => {}
        }
    }
    results.sort_by(|a, b| a.pak_name.cmp(&b.pak_name));
    Ok(results)
}

/// Scan `~mods` and return paks that contain any `.ini` file.
pub(crate) fn scan_mod_paks_any_ini(
    game_root: &str,
    recursive: bool,
) -> Result<Vec<PakIniListing>, String> {
    let mods_dir = mods_dir(game_root);
    if !mods_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for rel_path in crate::mods::walk_mod_files(&mods_dir, recursive) {
        let path = mods_dir.join(&rel_path);
        if path.extension().and_then(|x| x.to_str()) != Some("pak") {
            continue;
        }
        match inspect_pak_for_any_ini(&path) {
            Ok(Some(info)) => results.push(info),
            Ok(None) => {}
            Err(_) => {}
        }
    }
    results.sort_by(|a, b| a.pak_name.cmp(&b.pak_name));
    Ok(results)
}

/// Read CVar values from pak INI files, merged in runtime priority order (lowest first,
/// highest overrides): BaseEngine, DefaultEngine, WindowsEngine, DeviceProfiles. The map
/// is keyed by lowercased CVar name so insert is O(1); a Vec/retain merge here was O(N^2)
/// and stalled multi-second on mod paks with full engine INI overrides.
pub(crate) fn read_pak_tweaks(pak_path: &str) -> Result<Vec<PakTweakState>, String> {
    let pak_path = Path::new(pak_path);
    let info = inspect_pak_for_ini(pak_path)?
        .ok_or_else(|| "No INI config files found in this pak.".to_string())?;

    let layers: [(Option<&String>, &str); 4] = [
        (info.base_engine_entry.as_ref(), "BaseEngine.ini"),
        (info.engine_ini_entry.as_ref(), "DefaultEngine.ini"),
        (info.windows_engine_entry.as_ref(), "WindowsEngine.ini"),
        (
            info.device_profiles_entry.as_ref(),
            "DefaultDeviceProfiles.ini",
        ),
    ];

    let mut merged: std::collections::HashMap<String, PakTweakState> =
        std::collections::HashMap::new();
    for (entry, label) in layers {
        let Some(entry) = entry else { continue };
        let content = extract_file_to_string(pak_path, entry)?;
        for var in parse_console_vars(&content, label) {
            merged.insert(var.key.to_ascii_lowercase(), var);
        }
    }

    Ok(merged.into_values().collect())
}

/// Detect active tweaks from pak INI content using the shared tweak detector.
pub(crate) fn detect_pak_tweaks(pak_path: &str) -> Result<Vec<TweakState>, String> {
    let merged = read_pak_tweaks(pak_path)?;
    let synthetic: String = merged
        .iter()
        .map(|s| format!("{}={}", s.key, s.value))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(crate::tweaks::detect_tweaks_unscoped(&synthetic))
}

/// Extract a single file from a pak as a UTF-8 string.
pub(crate) fn extract_pak_ini(pak_path: &str, entry: &str) -> Result<String, String> {
    extract_file_to_string(Path::new(pak_path), entry)
}
