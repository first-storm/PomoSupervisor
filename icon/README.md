# 图标源文件

- `PomoSupervisor.icon/` — Icon Composer 项目。`Assets/` 里是四层素材（Tomato、Dial、Leaf、Timer，1024 画布，共用同一坐标），侧栏从底到顶就是这个顺序；浅色背景 #FCE8E5。Tomato、Dial、Leaf 用系统 Liquid Glass 效果，Timer 保持实色（关掉该层 Effects）。
- `app-icon.svg` — 把四层按 macOS 图标网格（824 底板，圆角 185，背景 #FCE8E5）合成的平面图标
- `tray.svg` — 菜单栏模板图标（番茄 + 叶子轮廓，黑色单色），未开始和休息时用
- `tray-focus.svg` — 专注中的菜单栏图标（同一轮廓，番茄红，非模板）
- `tray-paused.svg` — 暂停 / 离开时的菜单栏模板图标

## 重新生成

Liquid Glass 图标（`src-tauri/icons/Assets.car`，需要 Xcode 26+）：

```sh
icon/build-liquid-glass.sh
```

平面图标和菜单栏图标（需要 `rsvg-convert`，没有的话 `nix shell nixpkgs#librsvg`）：

```sh
rsvg-convert -w 1024 -h 1024 icon/app-icon.svg -o /tmp/app-icon.png && pnpm tauri icon /tmp/app-icon.png   # 然后删掉生成的 android/ ios/
for n in tray tray-focus tray-paused; do rsvg-convert -w 44 -h 44 icon/$n.svg -o src-tauri/icons/$n.png; done
```
