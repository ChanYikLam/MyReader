# MyReader

一个用 Rust + Tauri 2 构建的 macOS **只读** Markdown 阅读器。

核心体验是「打开即成品」：窗口出现的第一帧就是完整渲染后的阅读视图——没有白屏、没有源码闪现、
没有主题闪变、没有公式从 LaTeX 原文跳成排版、没有字体/图片加载后的布局跳动。

## 功能

- **打开文件**：菜单「文件 → 打开…」（⌘O，过滤 `.md/.markdown/.mdown/.txt`）、拖拽到窗口、
  命令行参数、文件关联（Finder 双击 `.md`）、最近打开（最多 10 条）。
- **Markdown 渲染**：标题、段落、粗体/斜体/删除线、行内代码、链接、图片、分隔线、
  有序/无序/嵌套列表、任务列表、表格（含对齐）、嵌套引用、脚注、围栏代码块语法高亮
  （syntect，class-based，未知语言回退纯文本）、内联 HTML 转义（不执行）。
- **数学公式**：`$...$` 行内、`$$...$$` 块级；KaTeX 本地打包（CSS + 字体，离线可用）；
  `pmatrix`/`aligned`/`cases` 等完整排版；非法公式显示红色原文 + 错误提示且不中断文档。
- **深浅色**：跟随系统 / 浅色 / 深色三态（「显示 → 外观」菜单，或右上角 ◐ 按钮循环切换），
  手动选择持久化；系统外观切换实时跟随（仅跟随系统模式）；代码高亮/公式/表格/引用块同步切换。
- **阅读体验**：目录侧栏（h1–h3、点击跳转、滚动高亮、⌘B 折叠、状态持久化）、
  字号调节（⌘= / ⌘- / ⌘0，13–24px）、每文件滚动位置记忆（首帧即恢复）、窗口大小/位置记忆。

明确不做：编辑/保存/新建、源码视图、导出、云同步、插件/自定义 CSS、公式编号与 `\ref`、
`\(...\)` / `\[...\]` 定界符、Windows/Linux 支持。

## 快捷键

| 快捷键 | 功能 |
|---|---|
| ⌘O | 打开文件 |
| ⌘B | 显示/隐藏目录侧栏 |
| ⌘= / ⌘+ / ⌘- | 增大 / 减小字号 |
| ⌘0 | 恢复默认字号 |
| ⌘W | 隐藏窗口（点 Dock 图标恢复） |

## 技术栈

| 层 | 选型 |
|---|---|
| 应用壳 | Tauri 2（Rust） |
| Markdown | comrak（`math_dollars`、GFM 扩展） |
| 代码高亮 | syntect（class-based，明暗两套主题 CSS 按 `data-theme` 作用域） |
| 数学公式 | KaTeX 0.19（纯渲染库，本地打包） |
| 前端 | 原生 TypeScript + HTML + CSS，Vite 构建（无前端框架/Tailwind/UI 库） |
| 持久化 | tauri-plugin-store（主题/字号/侧栏/最近/滚动位置）+ tauri-plugin-window-state（窗口大小位置） |

架构与设计决策见 [SPEC.md](SPEC.md)。

## 安装（DMG）

```bash
npm run tauri build
open src-tauri/target/release/bundle/dmg/MyReader_0.1.0_aarch64.dmg   # 拖入「应用程序」
```

或直接安装构建产物：`cp -R src-tauri/target/release/bundle/macos/MyReader.app /Applications/`。
安装后可在 Finder 中双击任意 `.md` 文件用 MyReader 打开。

### 分发给他人（Gatekeeper 提示处理）

本应用没有 Apple Developer ID 证书，只做了 **ad-hoc 完整签名**（`signingIdentity: "-"`，
构建时自动生成 `_CodeSignature`）。通过微信/网盘等传给其他 Mac 后，系统会给文件加隔离属性，
首次打开可能提示「已损坏」或「无法验证开发者」——这是 macOS 的 Gatekeeper 拦截，不是文件损坏。

接收方任选一种处理（推荐先做第 3 条再做第一次双击）：

1. 右键 `MyReader.app` → 打开，若出现「仍要打开」则确认；
2. 打开「系统设置 → 隐私与安全性」，滚动到底部点「仍要打开」；
3. 终端执行（先拖入「应用程序」后执行，执行前不要先双击它）：

   ```bash
   sudo xattr -dr com.apple.quarantine /Applications/MyReader.app
   ```

分发包（含 `安装说明.txt` 与 `首次打开.command`）在 `dist/` 下（被 git 忽略，不入库）：

- `dist/MyReader-0.1.0-macOS-arm64.dmg`
- `dist/MyReader-0.1.0-macOS-arm64.zip`

> 彻底免除提示需要 Apple Developer ID 签名 + 公证（notarytool），需要 Apple Developer Program 账号。
> 当前构建仅含 arm64 架构；若接收方是 Intel Mac，需要额外构建 Universal 版本。

## 构建与运行

前置：macOS 11+、Node.js 20+、Rust stable、Xcode Command Line Tools。

```bash
npm install
npm run tauri build          # 产物：src-tauri/target/release/bundle/macos/MyReader.app
open src-tauri/target/release/bundle/macos/MyReader.app
```

开发调试：

```bash
npm run tauri dev            # Vite + Tauri 热更新
npm run typecheck            # tsc --noEmit
npm run build                # 前端构建（tsc + vite build）
cargo test  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

> 本仓库只提交 `Cargo.lock` 与 `package-lock.json` 以保证可复现构建；应用未配置远程仓库，
> 不执行 `git push`。

## 命令行用法

```bash
"MyReader.app/Contents/MacOS/myreader" path/to/file.md
```

（macOS 的 `open --args` 不会把参数传给应用是系统行为；需要命令行打开时直接运行上面的二进制。）

## 数据与设置

- 设置（主题/字号/侧栏/最近文件/滚动位置）：
  `~/Library/Application Support/com.myreader.app/settings.json`
- 窗口大小/位置：`~/Library/Application Support/com.myreader.app/.window-state.json`
- 删除上述文件即可恢复默认设置。

## 验收素材

`tests/fixtures/` 下为手写验收样例：

| 文件 | 覆盖内容 |
|---|---|
| [basic.md](tests/fixtures/basic.md) | 标题/行内元素/嵌套引用/嵌套列表/分隔线/多语言代码块 |
| [elements.md](tests/fixtures/elements.md) | 表格对齐/任务列表/高亮代码块/SVG/缺失图片/远程图片/脚注/删除线 |
| [math.md](tests/fixtures/math.md) | 行内/块级/pmatrix/aligned/cases/非法公式/`\$` 转义/代码域豁免 |
| [security.md](tests/fixtures/security.md) | 内联 HTML 转义/`javascript:` 链接剥离 |

手动验收建议：

1. `open -a MyReader.app tests/fixtures/elements.md`：查看看板元素与图片占位；
2. 打开 `math.md`：公式、矩阵、对齐、错误提示；断网后重开确认字体本地可用；
3. 编辑任意 md 写入 `<script>alert(1)</script>`、`[x](javascript:alert(1))`、
   `$\href{javascript:alert(1)}{x}$`，确认不执行、不跳转；
4. 退出后重新打开同一长文：窗口出现即在记忆滚动位置；
5. 拖拽文件、⌘O、Finder 双击 `.md` 三种打开方式；
6. 「显示 → 外观」切换三态并重启验证持久化；系统切深色验证实时跟随；
7. 冷启动录屏逐帧检查：首帧即完整正文、深色系统无白闪、含公式文档无 LaTeX 原文跳变。

## 已知限制

- 公式识别完全交给 comrak `math_dollars`（Pandoc 启发式）。`价格 $5 和 $6` 这类文本若被误判，
  请用 `\$` 转义；本应用不做启发式修补。
- 远程图片首帧策略：整体最多预加载 800 ms，超时用固定 16:10 占位盒
  （图片随后在盒内 `object-fit: contain` 显示，盒高不变）。
- 本地图片只加载 Markdown 中真实引用且存在的文件（缺失显示 alt 占位）；
  非法协议（`data:`、`file:` 等）不加载。
- 非法 UTF-8 文件以替换字符宽松解码；超过 32 MB 或含二进制内容的文件拒绝打开并显示错误页。
- 公式与图片等远多于 2000 个的极端文档未做懒渲染（当前策略为全部公式在 reveal 前渲染）。
- 仅构建/验证本机 arm64 架构；未做 universal 二进制。
