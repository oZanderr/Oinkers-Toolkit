//! Applies catalogue-driven edits and raw INI content saves to pak files in place.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::pak::profile::strip_mount_prefix;

use super::cvars::{IniType, apply_edits_to_ini, parse_console_vars};
use super::io::{inspect_pak_for_ini, with_unpacked_pak};
use super::{PakIniFileContent, PakIniInfo, PakIniTarget, PakTweakEdit};

/// Apply catalogue-driven edits to a pak's chosen INI file and repack in place.
///
/// All plain CVar edits are written to `target` (defaulting to DeviceProfiles when
/// present, else Engine). Engine-section settings can only live in Engine.ini, so
/// they always go there. An existing copy of an edited key in the sibling file is
/// kept in sync so neither file shadows the other; keys absent from the sibling are
/// never injected.
pub(crate) fn apply_pak_tweaks(
    pak_path: &str,
    edits: &[PakTweakEdit],
    target: Option<PakIniTarget>,
) -> Result<String, String> {
    let pak = Path::new(pak_path);
    let info = inspect_pak_for_ini(pak)?
        .ok_or_else(|| "No INI config files found in this pak.".to_string())?;
    let pak_name = info.pak_name.clone();
    let edit_count = edits.len();

    let resolved = resolve_target(&info, target)?;
    let sibling = sibling_of(resolved);

    with_unpacked_pak(pak, |temp_dir| {
        // Engine-section settings (e.g. ApplicationScale, MaxClientRate) are not
        // console variables and can only live in Engine.ini.
        let (engine_section_edits, plain_edits): (Vec<PakTweakEdit>, Vec<PakTweakEdit>) = edits
            .iter()
            .cloned()
            .partition(|e| e.engine_section.is_some());

        let target_entry = entry_for(&info, resolved)
            .ok_or("Target INI entry missing despite target resolution")?;
        apply_edits_to_file(temp_dir, target_entry, ini_type_for(resolved), &plain_edits)?;

        if !engine_section_edits.is_empty()
            && let Some(eng_entry) = info.engine_ini_entry.as_ref()
        {
            apply_edits_to_file(temp_dir, eng_entry, IniType::Engine, &engine_section_edits)?;
        }

        // Keep an existing copy of each edited CVar consistent in the sibling file.
        if let Some(sib_entry) = entry_for(&info, sibling) {
            sync_existing_keys(
                temp_dir,
                sib_entry,
                ini_type_for(sibling),
                source_label(sibling),
                &plain_edits,
            )?;
        }

        Ok(())
    })?;

    let label = if edit_count == 1 { "change" } else { "changes" };
    Ok(format!("Applied {edit_count} {label} to {pak_name}"))
}

/// Resolve the requested target to one the pak actually contains, falling back to
/// DeviceProfiles when present (it overrides Engine CVars at runtime), else Engine.
fn resolve_target(info: &PakIniInfo, target: Option<PakIniTarget>) -> Result<PakIniTarget, String> {
    let requested_present = matches!(
        target,
        Some(PakIniTarget::Engine) if info.has_engine_ini
    ) || matches!(
        target,
        Some(PakIniTarget::DeviceProfiles) if info.has_device_profiles
    );
    if let Some(t) = target
        && requested_present
    {
        return Ok(t);
    }
    if info.has_device_profiles {
        Ok(PakIniTarget::DeviceProfiles)
    } else if info.has_engine_ini {
        Ok(PakIniTarget::Engine)
    } else {
        Err("No INI config files found in this pak.".to_string())
    }
}

fn sibling_of(target: PakIniTarget) -> PakIniTarget {
    match target {
        PakIniTarget::Engine => PakIniTarget::DeviceProfiles,
        PakIniTarget::DeviceProfiles => PakIniTarget::Engine,
    }
}

fn entry_for(info: &PakIniInfo, target: PakIniTarget) -> Option<&String> {
    match target {
        PakIniTarget::Engine => info.engine_ini_entry.as_ref(),
        PakIniTarget::DeviceProfiles => info.device_profiles_entry.as_ref(),
    }
}

fn ini_type_for(target: PakIniTarget) -> IniType {
    match target {
        PakIniTarget::Engine => IniType::Engine,
        PakIniTarget::DeviceProfiles => IniType::DeviceProfiles,
    }
}

/// `parse_console_vars` switches parsing rules based on whether the source name
/// contains "DeviceProfiles", so the label must reflect the file kind.
fn source_label(target: PakIniTarget) -> &'static str {
    match target {
        PakIniTarget::Engine => "DefaultEngine.ini",
        PakIniTarget::DeviceProfiles => "DefaultDeviceProfiles.ini",
    }
}

/// Replace raw INI file contents in a pak and repack in place.
pub(crate) fn save_pak_ini(
    pak_path: &str,
    files: Vec<PakIniFileContent>,
) -> Result<String, String> {
    let pak = Path::new(pak_path);
    let file_count = files.len();

    with_unpacked_pak(pak, |temp_dir| {
        for file in &files {
            let rel = strip_mount_prefix(&file.entry);
            let dest = temp_dir.join(rel);
            fs::write(&dest, &file.content)
                .map_err(|e| format!("Failed to write {}: {}", dest.display(), e))?;
        }
        Ok(())
    })?;

    let pak_name = pak
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    Ok(format!("Saved {} file(s) to {}", file_count, pak_name))
}

/// Read an extracted pak INI, apply `edits`, and write it back.
fn apply_edits_to_file(
    temp_dir: &Path,
    entry: &str,
    ini_type: IniType,
    edits: &[PakTweakEdit],
) -> Result<(), String> {
    if edits.is_empty() {
        return Ok(());
    }
    let file = temp_dir.join(strip_mount_prefix(entry));
    let content = fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read extracted INI {}: {}", file.display(), e))?;
    let modified = apply_edits_to_ini(&content, edits, ini_type);
    fs::write(&file, &modified)
        .map_err(|e| format!("Failed to write modified INI {}: {}", file.display(), e))?;
    Ok(())
}

/// Apply only the edits whose key already exists in the sibling file, so the two
/// INIs stay consistent without injecting keys the sibling never had.
fn sync_existing_keys(
    temp_dir: &Path,
    entry: &str,
    ini_type: IniType,
    source_label: &str,
    edits: &[PakTweakEdit],
) -> Result<(), String> {
    if edits.is_empty() {
        return Ok(());
    }
    let file = temp_dir.join(strip_mount_prefix(entry));
    let content = fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read sibling INI {}: {}", file.display(), e))?;

    let present: HashSet<String> = parse_console_vars(&content, source_label)
        .into_iter()
        .map(|s| s.key.to_ascii_lowercase())
        .collect();
    let filtered: Vec<PakTweakEdit> = edits
        .iter()
        .filter(|e| present.contains(&e.key.to_ascii_lowercase()))
        .cloned()
        .collect();
    if filtered.is_empty() {
        return Ok(());
    }

    let modified = apply_edits_to_ini(&content, &filtered, ini_type);
    fs::write(&file, &modified)
        .map_err(|e| format!("Failed to write sibling INI {}: {}", file.display(), e))?;
    Ok(())
}
