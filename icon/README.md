# 图标源文件

- `layers/` — Icon Composer 分层素材（Tomato、Dial、Leaf、Timer，1024 画布）
- `app-icon.svg` — 把四层按 macOS 图标网格（824 底板，圆角 185，背景 #FCE8E5）合成的平面图标
- `tray.svg` — 菜单栏模板图标（番茄 + 叶子轮廓，黑色单色）

重新生成：

```sh
rsvg-convert -w 1024 -h 1024 icon/app-icon.svg -o /tmp/app-icon.png && pnpm tauri icon /tmp/app-icon.png   # 然后删掉生成的 android/ ios/
rsvg-convert -w 44 -h 44 icon/tray.svg -o src-tauri/icons/tray.png
```
