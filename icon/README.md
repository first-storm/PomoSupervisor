# 图标源文件

- `layers/` — Icon Composer 分层素材（Tomato、Dial、Leaf、Timer，1024 画布）
- `app-icon.svg` — 把四层按 macOS 图标网格（824 底板，圆角 185，背景 #FCE8E5）合成的平面图标
- `tray.svg` — 菜单栏模板图标（番茄 + 叶子轮廓，黑色单色），未开始和休息时用
- `tray-focus.svg` — 专注中的菜单栏图标（同一轮廓，番茄红，非模板）
- `tray-paused.svg` — 暂停 / 离开时的菜单栏模板图标

重新生成：

```sh
rsvg-convert -w 1024 -h 1024 icon/app-icon.svg -o /tmp/app-icon.png && pnpm tauri icon /tmp/app-icon.png   # 然后删掉生成的 android/ ios/
for n in tray tray-focus tray-paused; do rsvg-convert -w 44 -h 44 icon/$n.svg -o src-tauri/icons/$n.png; done
```
