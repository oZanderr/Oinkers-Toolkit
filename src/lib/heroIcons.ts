const FALLBACK_ID = 9999;

const iconModules = import.meta.glob<string>("../assets/hero/*.png", {
  eager: true,
  query: "?url",
  import: "default",
});

const ICON_BY_ID: Map<number, string> = (() => {
  const map = new Map<number, string>();
  for (const [path, url] of Object.entries(iconModules)) {
    const m = path.match(/(\d+)\.png$/);
    if (!m) continue;
    map.set(Number(m[1]), url);
  }
  return map;
})();

export function heroIconUrl(characterId: number): string {
  return ICON_BY_ID.get(characterId) ?? ICON_BY_ID.get(FALLBACK_ID) ?? "";
}

export function hasHeroIcon(characterId: number): boolean {
  return ICON_BY_ID.has(characterId);
}

// Mirrors `detect_in_path` in src-tauri/src/mods/heroes.rs. Keep behavior in sync
// with the Rust side and its test cases.
function parseDigits(s: string, len: number): number | null {
  if (s.length !== len) return null;
  for (let i = 0; i < len; i++) {
    const c = s.charCodeAt(i);
    if (c < 48 || c > 57) return null;
  }
  return Number(s);
}

export function detectHeroIdsInPath(path: string, knownIds: Set<number>): number[] {
  const normalized = path.replace(/\\/g, "/").toLowerCase();
  if (normalized.includes("/environment/")) return [];

  const segments = normalized.split("/");
  const hits = new Set<number>();
  const anchoredChars = new Set<number>();

  const inUiTextures = normalized.includes("/ui/textures/");
  // Non-character UI subdirs embed digit tokens that look like char/skin ids
  // but aren't hero-attributed (career achievement tiers, depot themes).
  const uiDenylisted = inUiTextures && segments.some((s) => s === "career" || s === "depot");

  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    if (seg === "characters" || seg === "abilitysystem") {
      const charId = parseDigits(segments[i + 1] ?? "", 4);
      if (charId !== null && knownIds.has(charId)) {
        hits.add(charId);
        anchoredChars.add(charId);
      }
    }
  }

  if (!uiDenylisted) {
    // When path is anchored to a character, only accept 7-digit skin tokens
    // belonging to that character. Filenames in shared VFX/Particles folders
    // often reference unrelated character ids (e.g. NS_1034300_*).
    // Multi-hero `and` separator filenames use 8-digit tokens and are handled
    // by the UI-textures window scan below, not this rule.
    for (const seg of segments) {
      for (const tok of seg.split(/[^0-9]/)) {
        const skinId = parseDigits(tok, 7);
        if (skinId === null) continue;
        const charId = Math.floor(skinId / 1000);
        if (!knownIds.has(charId)) continue;
        if (anchoredChars.size > 0 && !anchoredChars.has(charId)) continue;
        hits.add(charId);
      }
    }
  }

  // UI textures: a 4-digit char-id segment pins ownership
  // and suppresses filename scanning; mastery/ability filenames embed other
  // heroes' ids that would otherwise leak in. Otherwise slide a 4-digit window
  // per digit token, first catalogue hit wins per token to avoid incidental
  // substring overlap. `and` splits multi-hero tokens.
  if (inUiTextures && !uiDenylisted) {
    const segmentAnchors: number[] = [];
    for (const seg of segments) {
      const c = parseDigits(seg, 4);
      if (c !== null && knownIds.has(c)) segmentAnchors.push(c);
    }

    if (segmentAnchors.length > 0) {
      for (const c of segmentAnchors) hits.add(c);
    } else {
      for (const seg of segments) {
        for (const tok of seg.split(/[^0-9]/)) {
          if (tok.length < 4) continue;
          for (let start = 0; start <= tok.length - 4; start++) {
            const charId = parseDigits(tok.substring(start, start + 4), 4);
            if (charId !== null && knownIds.has(charId)) {
              hits.add(charId);
              break;
            }
          }
        }
      }
    }
  }

  return [...hits].sort((a, b) => a - b);
}
