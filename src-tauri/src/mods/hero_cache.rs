//! Persistent hero detection cache stored in the OS cache dir, separate from user prefs.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::mods::heroes::HeroMatch;

const FILE_NAME: &str = "hero_cache.json";

/// Bump when matching logic or cache entry shape changes so stale entries get discarded on load.
pub(crate) const MOD_HERO_CACHE_VERSION: u32 = 4;

/// Cached hero detection result for a mod, keyed by display name.
/// Invalidated when the mod's total size changes or the character catalogue
/// stamp moves past the value captured at scan time.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ModHeroCacheEntry {
    pub size_bytes: u64,
    /// `HeroCache::last_character_data_sync` value when this entry was computed.
    /// Stale entries (older catalogue) recompute against the current catalogue.
    #[serde(default)]
    pub catalogue_stamp: u64,
    pub heroes: Vec<HeroMatch>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct HeroCache {
    #[serde(default)]
    pub last_character_data_sync: u64,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub entries: HashMap<String, ModHeroCacheEntry>,
}

pub(crate) type HeroCacheState = Mutex<HeroCache>;

fn cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("rivals-toolkit").join(FILE_NAME))
}

impl HeroCache {
    pub(crate) fn load() -> Self {
        let mut cache = cache_path()
            .and_then(|p| std::fs::read_to_string(&p).ok())
            .and_then(|raw| serde_json::from_str::<HeroCache>(&raw).ok())
            .unwrap_or_default();

        if cache.version != MOD_HERO_CACHE_VERSION {
            cache.entries.clear();
            cache.version = MOD_HERO_CACHE_VERSION;
        }

        cache
    }

    pub(crate) fn save(&self) -> Result<(), String> {
        let path = cache_path().ok_or_else(|| "Could not resolve cache directory".to_string())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }
}
