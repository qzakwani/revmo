# revmo logo (2b — monogram)

Mark: lowercase monospace **r** with an amber square sitting on the baseline to its right (the block cursor). The square doubles as a full stop after the wordmark.

## Files
- revmo-icon-dark.svg — app icon, dark tile (default). 512 viewBox, radius 112 (~22%, macOS/GNOME-style). Export PNG at 16/32/48/64/128/256/512/1024.
- revmo-icon-light.svg — light tile variant for light docks if wanted.
- revmo-icon-square.svg — no corner radius, for platforms that mask their own shape (Windows, adaptive icons).
- revmo-tray-mono.svg — 16px monochrome tray/menubar glyph; uses currentColor, no tile. macOS: mark as template image.
- revmo-wordmark-dark.svg / -light.svg — "revmo" + square. Requires IBM Plex Mono installed or convert text to outlines before shipping.

## Colors
- Tile dark #0F1012 · ink #ECECEC · accent #F0A940 (oklch 0.78 0.14 60)
- Tile light #F7F6F2 · ink #0F1012 · accent #A86A1E (oklch 0.55 0.14 60)

## Geometry (512 grid)
- r stem: x 160, y 208→368, stroke 56, square caps
- r shoulder: from (160,288) arc to (248,208) then to (280,208), stroke 56
- cursor square: 56×56 at (312,312) — bottom edge aligned to stem bottom (368)
- Safe area: keep all ink inside 128→384

## Rules
- Square is always accent, never ink. Never tint the r amber.
- Do not add a chevron or any other glyph; the wordmark plus square is the full lockup.
- In UI (footer, about screen) render the wordmark as live text in IBM Plex Mono 500 with a square sized ~0.25em, offset 0.1em right of the "o", bottom-aligned to the baseline.
