# M6 逐帧验收手册

本手册用于集中完成验收标准中「需要真实窗口/录屏」的条目（6/7/8/9/10/11/13/14/15），
预计 5–8 分钟。所有命令在仓库根目录执行。

> 前置：`npm run tauri build` 已成功；下文 `APP="$PWD/src-tauri/target/release/bundle/macos/MyReader.app"`。
> **台前调度（Stage Manager）** 会把非活动窗口缩到左侧缩略图，导致自动录屏拍不到真实首帧。
> 建议临时关闭并在验收后恢复：
>
> ```bash
> defaults write com.apple.WindowManager GloballyEnabled -bool false && killall Dock   # 关闭
> defaults write com.apple.WindowManager GloballyEnabled -bool true  && killall Dock   # 恢复
> ```

## 0. 公共准备

```bash
cd ~/Desktop/GDMU/03_项目/Markdown阅读器
APP="$PWD/src-tauri/target/release/bundle/macos/MyReader.app"
SETTINGS="$HOME/Library/Application Support/com.myreader.app/settings.json"
cp "$SETTINGS" /tmp/myreader-settings.backup.json 2>/dev/null || true   # 备份现有设置
pkill -x myreader 2>/dev/null || true
```

录屏（60fps，8 秒）与抽帧：

```bash
record() {  # record <输出.mp4> <秒数>
  ffmpeg -hide_banner -loglevel error -f avfoundation -framerate 60 \
    -capture_cursor 0 -i "1:none" -t "$2" -pix_fmt yuv420p -y "$1"
}
frames() {  # frames <输出.mp4> <目录>
  rm -rf "$2" && mkdir -p "$2"
  ffmpeg -hide_banner -loglevel error -i "$1" -vf "fps=60,scale=1440:-1" -y "$2/f_%04d.png"
}
```

## 1. 浅色冷启动逐帧（验收 6）

```bash
pkill -x myreader 2>/dev/null; sleep 0.5
record /tmp/m6-light.mp4 8 & FFPID=$!
sleep 1.2; open -a "$APP" "$PWD/tests/fixtures/elements.md"; wait $FFPID
frames /tmp/m6-light.mp4 /tmp/m6-light-frames
```

逐帧检查：窗口出现的第一帧即完整正文（标题/表格/任务列表/代码高亮/图片），不得出现白屏、
源码文本、`$...$` 原文或主题闪变。可用 `ffmpeg` 导出任意帧：

```bash
ffmpeg -ss 3.0 -i /tmp/m6-light.mp4 -frames:v 1 /tmp/m6-light-t3.png
```

期望：从窗口首次出现那一帧起，内容已经完整（无渐进填充）。

## 2. 深色冷启动无白闪（验收 7）

不改系统外观，用持久化设置让应用以深色启动：

```bash
pkill -x myreader 2>/dev/null; sleep 0.5
mkdir -p "$(dirname "$SETTINGS")"
python3 - <<'PY'
import json, pathlib, os
path = pathlib.Path(os.path.expanduser("~/Library/Application Support/com.myreader.app/settings.json"))
data = json.loads(path.read_text()) if path.exists() else {}
data["theme.mode"] = "dark"
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(data, ensure_ascii=False))
PY
record /tmp/m6-dark.mp4 8 & FFPID=$!
sleep 1.2; open -a "$APP" "$PWD/tests/fixtures/elements.md"; wait $FFPID
frames /tmp/m6-dark.mp4 /tmp/m6-dark-frames
```

逐帧检查：窗口首次出现即为深色完整正文；出现前不得有白色/浅色窗口帧（系统白底闪屏）。

## 3. 含公式冷启动（验收 8 / 3 / 4 / 5）

```bash
pkill -x myreader 2>/dev/null; sleep 0.5
record /tmp/m6-math.mp4 8 & FFPID=$!
sleep 1.2; open -a "$APP" "$PWD/tests/fixtures/math.md"; wait $FFPID
frames /tmp/m6-math.mp4 /tmp/m6-math-frames
```

检查：① 首帧视口内公式已是排版结果，无 LaTeX 原文跳变；② `pmatrix`/`aligned`/`cases` 正确；
③ 代码块/行内代码里的 `$x$` 原样、`\$5` 显示 `$5`；④ 非法 `\frac{1}` 显示红色原文+提示且
后续段落正常；⑤ 断网后重复一次（`networksetup -setairportpower Wi-Fi off` 后恢复）确认字体本地加载。

## 4. 滚动记忆首帧恢复（验收 9）

```bash
pkill -x myreader 2>/dev/null; sleep 0.5
python3 - <<'PY'
import json, pathlib, os
path = pathlib.Path(os.path.expanduser("~/Library/Application Support/com.myreader.app/settings.json"))
data = json.loads(path.read_text()) if path.exists() else {}
target = str(pathlib.Path("/tmp/perf-1mb-real.md").resolve())
data.setdefault("scroll", {})[target] = {
    "top": 30000.0, "docHeight": 520000.0, "viewportWidth": 1000.0, "updatedAt": 1,
}
path.write_text(json.dumps(data, ensure_ascii=False))
PY
record /tmp/m6-scroll.mp4 8 & FFPID=$!
sleep 1.2; open -a "$APP" /tmp/perf-1mb-real.md; wait $FFPID
frames /tmp/m6-scroll.mp4 /tmp/m6-scroll-frames
```

检查：窗口出现的第一帧就在中部位置（滚动条不在顶部），且没有从顶部滚过去的动画帧。

## 5. 运行中切换文件无空白（验收 10）

```bash
pkill -x myreader 2>/dev/null; sleep 0.5
open -a "$APP" "$PWD/tests/fixtures/elements.md"; sleep 2
record /tmp/m6-switch.mp4 6 & FFPID=$!
sleep 1.0; open -a "$APP" "$PWD/tests/fixtures/basic.md"; wait $FFPID
frames /tmp/m6-switch.mp4 /tmp/m6-switch-frames
```

检查：从旧内容到新内容之间的帧里，不得出现空白页/纯背景中间态。

## 6. 主题与窗口状态（验收 11，手动交互）

应用保持运行，手动验证：

1. 显示 → 外观：切换「深色/浅色/跟随系统」，界面、代码高亮、公式、表格、引用块同步变化；
2. 选「浅色」后让系统切深色（系统设置 → 外观）：应用保持浅色；
3. 退出并重新打开：主题保持上次选择；窗口大小/位置恢复；
4. 侧栏 ⌘B 折叠、⌘= / ⌘- / ⌘0 字号、关闭窗口（⌘W）后点 Dock 图标恢复窗口。

## 7. 链接与安全（验收 12/13）

```bash
open -a "$APP" "$PWD/tests/fixtures/security.md"   # <script> 显示为文本、javascript: 链接不跳
open -a "$APP" "$PWD/tests/fixtures/math.md"       # \href{javascript:...} 不产生可点链接
open -a "$APP" "$PWD/tests/fixtures/elements.md"   # 点击 https 链接 → 系统浏览器打开，应用窗口不变
```

## 8. 性能（验收 14）

`/tmp/perf-1mb-real.md`（约 1.7MB）与 `/tmp/perf-200-math.md`（200 公式）为临时生成的性能夹具
（不入库）。从录屏帧号换算启动耗时：窗口首次出现帧号 ÷ 60 ≈ 冷启动秒数。

```bash
for f in /tmp/perf-1mb-real.md /tmp/perf-200-math.md; do
  pkill -x myreader 2>/dev/null; sleep 0.5
  record "/tmp/m6-perf-$(basename "$f").mp4" 10 & FFPID=$!
  sleep 1.2; open -a "$APP" "$f"; wait $FFPID
done
```

期望：1MB 文档首屏 < 1.5s；200 公式视口内全部就绪 < 1.5s（本实现为 reveal 前全量渲染，
视口外无需补齐）；滚动过程不掉帧（目视/抽帧）。

## 9. 文件关联（验收 15）

`open -a "$APP" <file.md>` 即 LaunchServices 打开文件路径（与 Finder 双击等价）。
另请在 Finder 中双击一个 `.md` 文件确认由 MyReader 打开，并满足验收 6 的首帧要求。

## 10. 收尾

```bash
pkill -x myreader 2>/dev/null
cp /tmp/myreader-settings.backup.json "$SETTINGS" 2>/dev/null || true   # 恢复原设置（可选）
defaults write com.apple.WindowManager GloballyEnabled -bool true && killall Dock  # 恢复台前调度
```

## 验收映射

| 验收 | 本章节 | 证据 |
|---|---|---|
| 3/4/5 | §3 | 录屏 + 窗口截图 |
| 6/7 | §1/§2 | 逐帧录屏 |
| 8 | §3 | 逐帧录屏 |
| 9 | §4 | 逐帧录屏 |
| 10 | §5 | 录屏抽帧 |
| 11 | §6 | 手动交互 |
| 12/13 | §7 | 窗口截图 + 交互 |
| 14 | §8 | 帧号计时 |
| 15 | §9 | Finder 实测 |

## Windows 附加验收

在 Windows 10/11 上安装 `src-tauri/target/release/bundle/nsis/MyReader_0.1.0_x64-setup.exe` 后检查：

1. 启动不黑屏：`http://tauri.localhost` 的首屏导航未被守卫拦截，窗口 reveal 后是完整正文。
2. 文件关联：资源管理器中双击 `.md` 由 MyReader 打开；应用已运行时再次双击，复用同一窗口并载入新文件（单实例转发）。
3. 标题栏显示当前文件名，且不含 `\\?\` 前缀；最近文件列表中的路径同样正常。
4. 含 `images\a.png` 反斜杠图片路径的文档能正常加载本地图片。
5. 快捷键：Ctrl+O 打开、Ctrl+B 侧栏、Ctrl+= / Ctrl+- / Ctrl+0 字号。
6. 菜单「帮助 → 关于」弹出带版本号的信息框；「文件 → 退出」可退出应用。
7. 深色模式跟随系统、窗口大小位置记忆、拖拽打开、高 DPI 缩放显示正常。
