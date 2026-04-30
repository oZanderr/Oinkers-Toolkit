import { useEffect, useState } from "react";

import { invoke } from "@tauri-apps/api/core";

const target = new EventTarget();
const EVENT_NAME = "show-hero-icons-changed";

export async function setShowHeroIcons(enabled: boolean): Promise<void> {
  await invoke("set_show_hero_icons", { enabled });
  target.dispatchEvent(new CustomEvent<boolean>(EVENT_NAME, { detail: enabled }));
}

export function useShowHeroIcons(): boolean {
  const [value, setValue] = useState<boolean>(true);

  useEffect(() => {
    let cancelled = false;
    invoke<boolean>("get_show_hero_icons")
      .then((v) => {
        if (!cancelled) setValue(v);
      })
      .catch(() => {});
    const listener = (ev: Event) => {
      setValue((ev as CustomEvent<boolean>).detail);
    };
    target.addEventListener(EVENT_NAME, listener);
    return () => {
      cancelled = true;
      target.removeEventListener(EVENT_NAME, listener);
    };
  }, []);

  return value;
}
