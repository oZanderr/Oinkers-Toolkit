//! INI parsing and CVar edit application for pak-embedded config files.

use super::{PakTweakEdit, PakTweakState};

#[derive(Clone, Copy)]
pub(super) enum IniType {
    Engine,
    DeviceProfiles,
}

/// Apply edits to INI content.
pub(super) fn apply_edits_to_ini(
    content: &str,
    edits: &[PakTweakEdit],
    ini_type: IniType,
) -> String {
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    match ini_type {
        IniType::DeviceProfiles => {
            apply_device_profiles_edits(&mut lines, edits);
        }
        IniType::Engine => {
            apply_engine_edits(&mut lines, edits);
        }
    }

    let mut result = lines.join("\r\n");
    if !result.ends_with("\r\n") {
        result.push_str("\r\n");
    }
    result
}

/// Parse CVar key/value lines from Engine or DeviceProfiles INI content.
pub(super) fn parse_console_vars(content: &str, source: &str) -> Vec<PakTweakState> {
    let mut vars = Vec::new();
    let is_device_profiles = source.contains("DeviceProfiles");

    if is_device_profiles {
        let mut in_section = false;
        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with('[') {
                in_section = is_windows_device_profile_header(trimmed);
                continue;
            }

            if !in_section || trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }

            if let Some(kv) = parse_cvar_line(trimmed) {
                vars.push(PakTweakState {
                    key: kv.0,
                    value: kv.1,
                    source: source.to_string(),
                });
            }
        }
    } else {
        // Engine.ini keys can be outside [ConsoleVariables], so scan all sections.
        let mut in_any_section = false;
        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with('[') {
                in_any_section = true;
                continue;
            }

            if !in_any_section || trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }

            if let Some(kv) = parse_cvar_line(trimmed) {
                vars.push(PakTweakState {
                    key: kv.0,
                    value: kv.1,
                    source: source.to_string(),
                });
            }
        }
    }
    vars
}

/// Parse one CVar line, supporting optional `+CVars=` prefix.
fn parse_cvar_line(line: &str) -> Option<(String, String)> {
    let inner = if line.to_ascii_lowercase().starts_with("+cvars=") {
        &line["+CVars=".len()..]
    } else {
        line
    };

    let (key, value) = inner.split_once('=')?;
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}

/// Check whether a section header is `[Windows DeviceProfile]`.
fn is_windows_device_profile_header(header: &str) -> bool {
    header
        .trim()
        .eq_ignore_ascii_case("[Windows DeviceProfile]")
}

/// Remove non-comment CVar lines whose key matches `key_lower`.
fn remove_cvar_key(lines: &mut Vec<String>, key_lower: &str) {
    lines.retain(|line| {
        let t = line.trim();
        if t.starts_with(';') {
            return true;
        }
        match parse_cvar_line(t) {
            Some((k, _)) => k.to_ascii_lowercase() != key_lower,
            None => true,
        }
    });
}

/// Remove matching CVar lines only within the section starting at `section_start` (header index).
/// Stops at the next section header or end of file.
fn remove_cvar_key_in_section(lines: &mut Vec<String>, section_start: usize, key_lower: &str) {
    let mut i = section_start + 1;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with('[') {
            break;
        }
        if t.starts_with(';') || t.is_empty() {
            i += 1;
            continue;
        }
        match parse_cvar_line(t) {
            Some((k, _)) if k.to_ascii_lowercase() == key_lower => {
                lines.remove(i);
            }
            _ => {
                i += 1;
            }
        }
    }
}

/// Format a CVar assignment line.
fn format_cvar_line(key: &str, val: &str, preserve_prefix: bool) -> String {
    if preserve_prefix {
        format!("+CVars={}={}", key, val)
    } else {
        format!("{}={}", key, val)
    }
}

/// Find the end of a section (next header or EOF).
fn find_section_end(lines: &[String], section_start: usize) -> usize {
    for (i, line) in lines.iter().enumerate().skip(section_start + 1) {
        if line.trim().starts_with('[') {
            return i;
        }
    }
    lines.len()
}

/// Find an insert point near the end of a section, before trailing blank lines.
fn find_section_insert_point(lines: &[String], section_start: usize) -> usize {
    let end = find_section_end(lines, section_start);
    let mut insert = end;
    while insert > section_start + 1 && lines[insert - 1].trim().is_empty() {
        insert -= 1;
    }
    insert
}

/// Apply edits inside the `[Windows DeviceProfile]` section.
fn apply_device_profiles_edits(lines: &mut Vec<String>, edits: &[PakTweakEdit]) {
    let mut section_start: Option<usize> = None;
    let mut section_end: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if is_windows_device_profile_header(trimmed) {
            section_start = Some(i);
        } else if trimmed.starts_with('[') && section_start.is_some() && section_end.is_none() {
            section_end = Some(i);
        }
    }

    // No section yet: create it so value inserts land somewhere. Skip when there's
    // nothing to insert (pure removals) to avoid leaving an empty section behind.
    let start = match section_start {
        Some(start) => start,
        None => {
            if !edits.iter().any(|e| e.value.is_some()) {
                return;
            }
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push("[Windows DeviceProfile]".to_string());
            lines.len() - 1
        }
    };
    let end = section_end.unwrap_or(lines.len());

    for edit in edits {
        let key_lower = edit.key.to_ascii_lowercase();

        let mut found_idx = None;
        for (i, line) in lines
            .iter()
            .enumerate()
            .take(end.min(lines.len()))
            .skip(start + 1)
        {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                break;
            }
            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }
            if let Some((k, _)) = parse_cvar_line(trimmed)
                && k.to_ascii_lowercase() == key_lower
            {
                found_idx = Some(i);
                break;
            }
        }

        match (&edit.value, found_idx) {
            (Some(val), Some(_)) => {
                let end_now = section_end.unwrap_or(lines.len());
                for i in (start + 1)..end_now.min(lines.len()) {
                    let t = lines[i].trim().to_string();
                    if t.starts_with(';') || t.is_empty() {
                        continue;
                    }
                    if let Some((k, _)) = parse_cvar_line(&t)
                        && k.to_ascii_lowercase() == key_lower
                    {
                        let has_prefix = t.to_ascii_lowercase().starts_with("+cvars=");
                        lines[i] = format_cvar_line(&edit.key, val, has_prefix);
                    }
                }
            }
            (Some(val), None) => {
                let insert_at = find_section_insert_point(lines, start);
                lines.insert(insert_at, format_cvar_line(&edit.key, val, true));
            }
            (None, Some(_)) => {
                remove_cvar_key_in_section(lines, start, &key_lower);
            }
            (None, None) => {}
        }
    }
}

/// Apply edits to Engine.ini.
///
/// Existing keys are updated in place. New keys are inserted into `engine_section`
/// when provided, otherwise into `[ConsoleVariables]`.
fn apply_engine_edits(lines: &mut Vec<String>, edits: &[PakTweakEdit]) {
    for edit in edits {
        let key_lower = edit.key.to_ascii_lowercase();

        let mut in_section = false;
        let mut found_idx: Option<usize> = None;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_section = true;
                continue;
            }
            if !in_section || trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }
            if let Some((k, _)) = parse_cvar_line(trimmed)
                && k.to_ascii_lowercase() == key_lower
            {
                found_idx = Some(i);
                break;
            }
        }

        match (&edit.value, found_idx) {
            (Some(val), Some(_)) => {
                let new_line = format_cvar_line(&edit.key, val, false);
                for line in lines.iter_mut() {
                    let t = line.trim();
                    if t.starts_with(';') {
                        continue;
                    }
                    if let Some((k, _)) = parse_cvar_line(t)
                        && k.to_ascii_lowercase() == key_lower
                    {
                        *line = new_line.clone();
                    }
                }
            }
            (None, Some(_)) => {
                remove_cvar_key(lines, &key_lower);
            }
            (None, None) => {}
            (Some(val), None) => {
                let target_header = edit
                    .engine_section
                    .as_deref()
                    .map(|s| format!("[{}]", s))
                    .unwrap_or_else(|| "[ConsoleVariables]".to_string());

                let section_start = lines
                    .iter()
                    .rposition(|l| l.trim().eq_ignore_ascii_case(&target_header));

                let section_start = match section_start {
                    Some(idx) => idx,
                    None => {
                        if !lines.last().is_some_and(|l| l.trim().is_empty()) {
                            lines.push(String::new());
                        }
                        lines.push(target_header);
                        lines.len() - 1
                    }
                };

                let insert_at = find_section_insert_point(lines, section_start);
                lines.insert(insert_at, format_cvar_line(&edit.key, val, false));
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod roundtrip_tests {
    //! Round-trip catalogue verification: simulate the full apply detect pipeline
    //! at the INI layer (skipping pak encrypt/repack which is repak's responsibility).
    //! For each catalogue tweak, toggle ON via the same logic the frontend uses,
    //! apply the edits like apply_pak_tweaks does, then run detect_tweaks_unscoped
    //! against the merged result. Catches regressions in detection vs apply drift.
    use super::*;
    use crate::pak_tweaks::{PakIniTarget, PakTweakEdit};
    use crate::tweaks::TweakState;
    use crate::tweaks::catalogue::{TweakDefinition, TweakKind, tweak_catalogue};
    use crate::tweaks::detect_tweaks_unscoped;
    use std::collections::HashSet;

    /// Mirrors the frontend `toggleQuickTweak` logic for the ON case.
    fn edits_for_on(def: &TweakDefinition) -> Vec<PakTweakEdit> {
        match &def.kind {
            TweakKind::RemoveLines { lines, .. } => lines
                .iter()
                .map(|line| {
                    let key = line
                        .pattern
                        .split_once('=')
                        .map(|(k, _)| k)
                        .unwrap_or(&line.pattern)
                        .to_string();
                    let replace_val: Option<String> = line.replace_with.as_ref().map(|rw| {
                        rw.split_once('=')
                            .map(|(_, v)| v.to_string())
                            .unwrap_or_else(|| rw.clone())
                    });
                    PakTweakEdit {
                        key,
                        value: replace_val, // None for plain remove, Some for replace_with
                        engine_section: line.engine_section.clone(),
                    }
                })
                .collect(),
            TweakKind::Toggle {
                key,
                on_value,
                engine_section,
                ..
            } => vec![PakTweakEdit {
                key: key.clone(),
                value: Some(on_value.clone()),
                engine_section: engine_section.clone(),
            }],
            TweakKind::Slider {
                key,
                default_value,
                engine_section,
                ..
            } => {
                // Pick a non-default value so detection registers as active.
                let v = if (*default_value - 0.0).abs() < f64::EPSILON {
                    "1".to_string()
                } else {
                    "0".to_string()
                };
                vec![PakTweakEdit {
                    key: key.clone(),
                    value: Some(v),
                    engine_section: engine_section.clone(),
                }]
            }
            TweakKind::BatchToggle { entries, .. } => entries
                .iter()
                .map(|e| PakTweakEdit {
                    key: e.key.clone(),
                    value: Some(e.on_value.clone()),
                    engine_section: e.engine_section.clone(),
                })
                .collect(),
        }
    }

    /// Apply edits to engine+dp content mirroring `apply_pak_tweaks`: plain CVar edits
    /// go to `target`, engine-section settings always go to Engine, and an existing copy
    /// of an edited key in the sibling file is kept in sync (never injected).
    fn apply_to_pair(
        engine: &str,
        dp: &str,
        edits: &[PakTweakEdit],
        target: PakIniTarget,
    ) -> (String, String) {
        let (engine_section_edits, plain_edits): (Vec<_>, Vec<_>) = edits
            .iter()
            .cloned()
            .partition(|e| e.engine_section.is_some());

        let mut engine_after = engine.to_string();
        let mut dp_after = dp.to_string();

        // Plain CVar edits -> the chosen target file.
        match target {
            PakIniTarget::Engine => {
                engine_after = apply_edits_to_ini(&engine_after, &plain_edits, IniType::Engine);
            }
            PakIniTarget::DeviceProfiles => {
                dp_after = apply_edits_to_ini(&dp_after, &plain_edits, IniType::DeviceProfiles);
            }
        }

        // Engine-section settings are not console variables; they only live in Engine.ini.
        if !engine_section_edits.is_empty() {
            engine_after =
                apply_edits_to_ini(&engine_after, &engine_section_edits, IniType::Engine);
        }

        // Keep an existing copy of each edited CVar consistent in the sibling file.
        let sibling_is_engine = matches!(target, PakIniTarget::DeviceProfiles);
        let (sibling_content, sibling_label, sibling_type) = if sibling_is_engine {
            (&engine_after, "DefaultEngine.ini", IniType::Engine)
        } else {
            (
                &dp_after,
                "DefaultDeviceProfiles.ini",
                IniType::DeviceProfiles,
            )
        };
        let present: HashSet<String> = parse_console_vars(sibling_content, sibling_label)
            .into_iter()
            .map(|s| s.key.to_ascii_lowercase())
            .collect();
        let filtered: Vec<PakTweakEdit> = plain_edits
            .iter()
            .filter(|e| present.contains(&e.key.to_ascii_lowercase()))
            .cloned()
            .collect();
        if sibling_is_engine {
            engine_after = apply_edits_to_ini(&engine_after, &filtered, sibling_type);
        } else {
            dp_after = apply_edits_to_ini(&dp_after, &filtered, sibling_type);
        }

        (engine_after, dp_after)
    }

    /// Build the synthetic key=value content `detect_pak_tweaks` feeds to the detector.
    fn merge_to_synthetic(engine: &str, dp: &str) -> String {
        let mut merged = parse_console_vars(engine, "DefaultEngine.ini");
        let dp_vars = parse_console_vars(dp, "DefaultDeviceProfiles.ini");
        for dp_var in dp_vars {
            let key_lower = dp_var.key.to_ascii_lowercase();
            merged.retain(|v| v.key.to_ascii_lowercase() != key_lower);
            merged.push(dp_var);
        }
        merged
            .iter()
            .map(|s| format!("{}={}", s.key, s.value))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn state_for<'a>(states: &'a [TweakState], id: &str) -> &'a TweakState {
        states
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("state for {id} missing"))
    }

    #[test]
    fn fix_abilities_with_replace_with_round_trip() {
        // The exact regression that prompted this audit: r.CustomDepth=0 → r.CustomDepth=3.
        let cat = tweak_catalogue();
        let def = cat
            .iter()
            .find(|t| t.id == "fix_abilities")
            .expect("fix_abilities catalogue entry");

        // Pak baseline: engine.ini contains the OFF-state lines (typical mod state).
        let engine = "[ConsoleVariables]\nr.PostProcessing.DisableMaterials=1\nr.CustomDepth=0\nr.LightTile.Enable=0\n";
        let dp = "[Windows DeviceProfile]\n";

        // Detect baseline: tweak should read OFF (original patterns present).
        let detect_off = detect_tweaks_unscoped(&merge_to_synthetic(engine, dp));
        assert!(
            !state_for(&detect_off, "fix_abilities").active,
            "OFF baseline should detect inactive"
        );

        // Apply ON.
        let edits = edits_for_on(def);
        let (engine_on, dp_on) = apply_to_pair(engine, dp, &edits, PakIniTarget::DeviceProfiles);

        // After ON: PostProcessing.DisableMaterials and LightTile.Enable removed,
        // CustomDepth replaced with 3.
        let detect_on = detect_tweaks_unscoped(&merge_to_synthetic(&engine_on, &dp_on));
        assert!(
            state_for(&detect_on, "fix_abilities").active,
            "ON state must be detected after apply (regression: was reading inactive due to key-only check)"
        );
    }

    #[test]
    fn batch_toggle_network_revert_round_trip() {
        let cat = tweak_catalogue();
        let def = cat
            .iter()
            .find(|t| t.id == "network_revert_update_65")
            .expect("network_revert_update_65 catalogue entry");

        let engine = "";
        let dp = "[Windows DeviceProfile]\n";

        let edits = edits_for_on(def);
        let (engine_on, dp_on) = apply_to_pair(engine, dp, &edits, PakIniTarget::DeviceProfiles);

        let detect_on = detect_tweaks_unscoped(&merge_to_synthetic(&engine_on, &dp_on));
        assert!(
            state_for(&detect_on, "network_revert_update_65").active,
            "BatchToggle ON state must be detected"
        );
    }

    #[test]
    fn slider_write_default_on_disable_round_trip() {
        let cat = tweak_catalogue();
        let def = cat
            .iter()
            .find(|t| {
                matches!(
                    &t.kind,
                    TweakKind::Slider {
                        write_default_on_disable: true,
                        ..
                    }
                )
            })
            .expect("at least one slider with write_default_on_disable=true");

        let engine = "[ConsoleVariables]\n";
        let dp = "[Windows DeviceProfile]\n";

        let edits = edits_for_on(def);
        let (engine_on, dp_on) = apply_to_pair(engine, dp, &edits, PakIniTarget::DeviceProfiles);

        let detect_on = detect_tweaks_unscoped(&merge_to_synthetic(&engine_on, &dp_on));
        assert!(
            state_for(&detect_on, &def.id).active,
            "slider non-default value must register as active"
        );
    }

    /// Walk every catalogue tweak and verify the apply→detect round-trip.
    /// This is the safety net the regression slipped past.
    #[test]
    fn full_catalogue_apply_detect_round_trip() {
        let cat = tweak_catalogue();

        // Baseline pak content with original RemoveLines patterns present so
        // that those tweaks start in the OFF state.
        let mut engine = String::from("[ConsoleVariables]\n");
        for def in cat.iter() {
            if let TweakKind::RemoveLines { lines, .. } = &def.kind {
                for line in lines {
                    engine.push_str(&line.pattern);
                    engine.push('\n');
                }
            }
        }
        let dp = String::from("[Windows DeviceProfile]\n");

        // Both edit targets must round-trip to ACTIVE: the merged runtime view is the
        // same regardless of which file holds the CVar.
        for target in [PakIniTarget::DeviceProfiles, PakIniTarget::Engine] {
            for def in cat.iter() {
                let edits = edits_for_on(def);
                if edits.is_empty() {
                    continue;
                }
                let (engine_after, dp_after) = apply_to_pair(&engine, &dp, &edits, target);
                let states = detect_tweaks_unscoped(&merge_to_synthetic(&engine_after, &dp_after));
                let state = state_for(&states, &def.id);
                assert!(
                    state.active,
                    "tweak {} should detect ACTIVE after applying ON edits (target={:?}, kind={:?})",
                    def.id,
                    target,
                    std::mem::discriminant(&def.kind)
                );
            }
        }
    }

    fn cvar_edit(key: &str, value: Option<&str>) -> PakTweakEdit {
        PakTweakEdit {
            key: key.into(),
            value: value.map(str::to_string),
            engine_section: None,
        }
    }

    #[test]
    fn target_engine_syncs_existing_key_in_device_profiles() {
        let engine = "[ConsoleVariables]\nr.Foo=0\n";
        let dp = "[Windows DeviceProfile]\n+CVars=r.Foo=0\n";
        let (engine_after, dp_after) = apply_to_pair(
            engine,
            dp,
            &[cvar_edit("r.Foo", Some("1"))],
            PakIniTarget::Engine,
        );
        assert!(
            engine_after.contains("r.Foo=1"),
            "engine target:\n{engine_after}"
        );
        assert!(
            dp_after.contains("+CVars=r.Foo=1"),
            "sibling DeviceProfiles copy should update so it doesn't shadow:\n{dp_after}"
        );
    }

    #[test]
    fn target_device_profiles_syncs_existing_key_in_engine() {
        let engine = "[ConsoleVariables]\nr.Foo=0\n";
        let dp = "[Windows DeviceProfile]\n+CVars=r.Foo=0\n";
        let (engine_after, dp_after) = apply_to_pair(
            engine,
            dp,
            &[cvar_edit("r.Foo", Some("1"))],
            PakIniTarget::DeviceProfiles,
        );
        assert!(
            dp_after.contains("+CVars=r.Foo=1"),
            "dp target:\n{dp_after}"
        );
        assert!(
            engine_after.contains("r.Foo=1"),
            "sibling Engine copy should stay consistent:\n{engine_after}"
        );
    }

    #[test]
    fn removal_clears_key_from_both_files() {
        let engine = "[ConsoleVariables]\nr.Foo=0\n";
        let dp = "[Windows DeviceProfile]\n+CVars=r.Foo=0\n";
        let (engine_after, dp_after) = apply_to_pair(
            engine,
            dp,
            &[cvar_edit("r.Foo", None)],
            PakIniTarget::DeviceProfiles,
        );
        assert!(
            !engine_after.to_ascii_lowercase().contains("r.foo"),
            "engine copy should be removed for a true reset:\n{engine_after}"
        );
        assert!(
            !dp_after.to_ascii_lowercase().contains("r.foo"),
            "dp copy should be removed:\n{dp_after}"
        );
    }

    #[test]
    fn sibling_without_key_is_not_injected() {
        let engine = "[ConsoleVariables]\n";
        let dp = "[Windows DeviceProfile]\n";
        let (engine_after, dp_after) = apply_to_pair(
            engine,
            dp,
            &[cvar_edit("r.Foo", Some("1"))],
            PakIniTarget::DeviceProfiles,
        );
        assert!(
            dp_after.contains("+CVars=r.Foo=1"),
            "target dp gets the key:\n{dp_after}"
        );
        assert!(
            !engine_after.to_ascii_lowercase().contains("r.foo"),
            "engine never had the key, so it must not be injected:\n{engine_after}"
        );
    }

    #[test]
    fn device_profiles_section_created_when_missing() {
        let dp = "; device profiles file with no windows section\n";
        let out = apply_edits_to_ini(
            dp,
            &[cvar_edit("r.Foo", Some("1"))],
            IniType::DeviceProfiles,
        );
        assert!(
            out.contains("[Windows DeviceProfile]"),
            "missing section should be created:\n{out}"
        );
        assert!(
            out.contains("+CVars=r.Foo=1"),
            "cvar should be inserted:\n{out}"
        );
    }
}
