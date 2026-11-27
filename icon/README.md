# Icon sources

- `PomoSupervisor.icon/` is the Icon Composer project. `Assets/` holds its four layers (Tomato, Dial, Leaf, Timer) on a shared 1024 canvas, in that order from bottom to top in the sidebar. The light background is #FCE8E5. Tomato, Dial and Leaf use the system Liquid Glass effect; Timer stays a solid colour, with Effects off for that layer.
- `app-icon.svg` is the flat icon: the four layers composed on the macOS icon grid (an 824 plate, corner radius 185, background #FCE8E5).
- `tray.svg` is the menu-bar template icon (the outline of the tomato and leaf, in black), shown before a round and during breaks.
- `tray-focus.svg` is the menu-bar icon while focusing (the same outline in tomato red, not a template).
- `tray-paused.svg` is the menu-bar template icon while paused or away.

## Regenerating

The Liquid Glass icon (`src-tauri/icons/Assets.car`; needs Xcode 26+):

```sh
icon/build-liquid-glass.sh
```

The flat icon and the menu-bar icons (needs `rsvg-convert`; without it, `nix shell nixpkgs#librsvg`):

```sh
rsvg-convert -w 1024 -h 1024 icon/app-icon.svg -o /tmp/app-icon.png && pnpm tauri icon /tmp/app-icon.png   # then delete the generated android/ and ios/
for n in tray tray-focus tray-paused; do rsvg-convert -w 44 -h 44 icon/$n.svg -o src-tauri/icons/$n.png; done
```
