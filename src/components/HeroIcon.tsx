import * as React from "react";

import { Tip } from "@/components/ui/tooltip";
import { heroIconUrl } from "@/lib/heroIcons";
import { cn } from "@/lib/utils";

interface HeroIconProps {
  characterId: number;
  name?: string;
  size?: number;
  tooltip?: React.ReactNode;
  className?: string;
}

export function HeroIcon({ characterId, name, size = 20, tooltip, className }: HeroIconProps) {
  const url = heroIconUrl(characterId);
  const alt = name ?? `Hero ${characterId}`;
  const img = (
    <img
      src={url}
      alt={alt}
      width={size}
      height={size}
      loading="lazy"
      decoding="async"
      className={cn(
        "shrink-0 rounded-full bg-secondary/40 ring-1 ring-border/50 object-cover",
        className
      )}
      style={{ width: size, height: size }}
    />
  );
  const content = tooltip ?? name;
  if (content === undefined || content === null || content === "") return img;
  return <Tip content={content}>{img}</Tip>;
}
