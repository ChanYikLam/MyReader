<div align="center">

<img src="src-tauri/icons/128x128@2x.png" width="128" alt="MyReader 图标" />

# MyReader

**macOS / Windows 只读 Markdown 阅读器 · 打开即成品**

第一帧就是完整排版的阅读视图：没有白屏、没有源码闪现、没有主题闪变、
没有公式从 LaTeX 原文跳成排版、没有字体或图片加载后的布局跳动。

[![macOS 11+](https://img.shields.io/badge/macOS-11%2B-000000?logo=apple&logoColor=white)](#快速开始)
[![Windows 10/11 (Beta)](https://img.shields.io/badge/Windows-10%2F11%20(Beta)-0078D6?logo=windows&logoColor=white)](#windows-版本状态beta)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-green)](#)
[![GitHub](https://img.shields.io/badge/GitHub-ChanYikLam%2FMyReader-181717?logo=github&logoColor=white)](https://github.com/ChanYikLam/MyReader)
[![CNB](https://img.shields.io/badge/CNB-YukinoNO1%2FMyReader-1A7AF8)](https://cnb.cool/YukinoNO1/MyReader)

[特性](#特性) · [Windows 状态](#windows-版本状态beta) · [快速开始](#快速开始) · [使用](#使用) ·
[技术栈](#技术栈) · [架构](#架构与项目结构) · [已知限制](#已知限制) · [许可证](#许可证)

</div>

---

> **English** — MyReader is a read-only Markdown reader for macOS and Windows built with
> Rust and Tauri 2. The window is only shown after the full document has been parsed,
> sanitized, highlighted and typeset, so the very first frame is already the finished
> reading view. Features GFM, KaTeX math, syntax highlighting, outline sidebar,
> light/dark theme and per-file scroll memory. See [SPEC.md](SPEC.md) for the design
> decisions.

## 特性

**打开方式**

- 菜单「文件 → 打开…」（⌘O / Ctrl+O），文件过滤 `.md / .markdown / .mdown / .txt`
- 拖拽文件到窗口、命令行参数、Finder / 资源管理器双击（文件关联）
- 最近打开列表（最多 10 条）

**Markdown 渲染**

- 标题、段落、粗体 / 斜体 / 删除线、行内代码、链接、图片、分隔线
- 有序 / 无序 / 嵌套列表、任务列表、表格（含对齐）、嵌套引用、脚注
- 围栏代码块语法高亮（syntect，class-based；未知语言回退纯文本）
- 内联 HTML 一律转义为文本，不执行

**数学公式**

- `$...$` 行内公式、`$$...$$` 块级公式
- KaTeX 本地打包（CSS + 字体），完全离线可用
- `pmatrix` / `aligned` / `cases` 等完整排版
- 非法公式显示红色原文与错误提示，不中断整篇文档

**阅读体验**

- 深浅色三态（跟随系统 / 浅色 / 深色），菜单或右上角按钮切换，手动选择持久化
- 系统外观切换实时跟随（仅跟随系统模式），代码高亮 / 公式 / 表格 / 引用块同步换色
- 目录侧栏（h1–h3、点击跳转、滚动高亮、折叠、状态持久化）
- 字号调节（⌘= / ⌘- / ⌘0，Windows 为 Ctrl+= / Ctrl+- / Ctrl+0，13–24px）
- 每个文件的滚动位置独立记忆，重新打开首帧即恢复
- 窗口大小 / 位置记忆，多显示器边界由插件处理

**明确不做**：编辑 / 保存 / 新建、源码视图、导出、云同步、插件与自定义 CSS、
公式编号与 `\ref`、`\(...\)` / `\[...\]` 定界符、Linux 支持。

桌面端支持 macOS 11+ 与 Windows 10/11。

## Windows 版本状态（Beta）

| 项目 | 状态 |
|---|---|
| 代码适配 | 完成：导航守卫、平台菜单、单实例文件关联、路径规范化、Ctrl 快捷键、字体栈 |
| macOS 回归验证 | 通过：79 个单元测试、clippy `-D warnings`、release 构建 |
| Windows CI 构建 | 已配置 [`.github/workflows/windows.yml`](.github/workflows/windows.yml)，待首次运行 |
| Windows 真机验收 | **待完成**（安装、文件关联、首帧、菜单、快捷键、高 DPI） |
| 代码签名 | 未做，首次运行可能出现 SmartScreen 提示 |

macOS 版本为稳定版；Windows 版本当前为 Beta，验收通过后会更新本表。

验收清单见 [docs/VERIFICATION.md](docs/VERIFICATION.md) 的「Windows 附加验收」一节。

## 快速开始

### 环境要求

| 依赖 | 版本 |
|---|---|
| macOS（构建与应用） | 11.0 及以上，Xcode Command Line Tools 最新版 |
| Windows（构建与应用） | 10 / 11，WebView2 运行时（系统自带或安装包内引导下载） |
| Node.js | 20+ |
| Rust | stable（含 `cargo`） |
| Windows 构建工具 | Visual Studio Build Tools（「使用 C++ 的桌面开发」） |

### 从源码构建

```bash
git clone https://github.com/ChanYikLam/MyReader.git
# CNB 镜像：git clone https://cnb.cool/YukinoNO1/MyReader.git
cd MyReader

npm install
npm run tauri build
```

构建产物：

| 平台 | 路径 |
|---|---|
| macOS | `src-tauri/target/release/bundle/macos/MyReader.app` |
| macOS | `src-tauri/target/release/bundle/dmg/MyReader_0.1.0_<arch>.dmg` |
| Windows | `src-tauri/target/release/bundle/nsis/MyReader_0.1.0_x64-setup.exe` |
| Windows | `src-tauri/target/release/bundle/msi/MyReader_0.1.0_x64_en-US.msi` |

Windows 也可以只构建 NSIS 安装包（不需要 WiX / VBSCRIPT）：

```powershell
npm run tauri build -- --bundles nsis
```

Windows 构建需要在本机 Windows 上完成。仓库提供 GitHub Actions 工作流
[`.github/workflows/windows.yml`](.github/workflows/windows.yml)，在 Actions 页手动
触发或推送 `v*` 标签即可得到 NSIS 安装包。

安装：

```bash
# macOS
cp -R src-tauri/target/release/bundle/macos/MyReader.app /Applications/
```

```powershell
# Windows：双击 MyReader_0.1.0_x64-setup.exe，按向导安装
```

安装后即可在 Finder / 资源管理器中双击任意 `.md` 文件用 MyReader 打开。

### 开发调试

```bash
npm run tauri dev      # Vite + Tauri 热更新
npm run typecheck      # tsc --noEmit
npm run build          # 前端构建（tsc + vite build）
```

### 测试与静态检查

```bash
cargo test  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt   --manifest-path src-tauri/Cargo.toml -- --check
```

Rust 侧包含 78 个单元测试，覆盖解析、净化、图片路径、设置、主题映射等纯函数逻辑；
界面行为与首帧表现按 [docs/VERIFICATION.md](docs/VERIFICATION.md) 的手册人工验收。

## 使用

### 快捷键

| macOS | Windows | 功能 |
|---|---|---|
| ⌘O | Ctrl+O | 打开文件 |
| ⌘B | Ctrl+B | 显示 / 隐藏目录侧栏 |
| ⌘= / ⌘+ / ⌘- | Ctrl+= / Ctrl++ / Ctrl+- | 增大 / 减小字号 |
| ⌘0 | Ctrl+0 | 恢复默认字号 |
| ⌘W | Ctrl+W | 隐藏窗口（点 Dock 图标 / 任务栏按钮恢复） |

### 命令行

```bash
# macOS
"MyReader.app/Contents/MacOS/myreader" path/to/file.md
```

```powershell
# Windows
& "$env:LOCALAPPDATA\Programs\MyReader\MyReader.exe" "C:\docs\file.md"
```

> macOS 的 `open --args` 不会把参数传给应用，这是系统行为；需要命令行打开时请直接运行
> 上面的二进制。

## 技术栈

| 层 | 选型 |
|---|---|
| 应用壳 | Tauri 2（Rust） |
| Markdown | [comrak](https://github.com/kivikakk/comrak)（`math_dollars`、GFM 扩展） |
| 代码高亮 | [syntect](https://github.com/trishume/syntect)（class-based，明暗两套主题按 `data-theme` 作用域） |
| 数学公式 | [KaTeX](https://katex.org) 0.19（纯渲染库，本地打包） |
| HTML 净化 | [ammonia](https://github.com/rust-ammonia/ammonia)（白名单） |
| 前端 | 原生 TypeScript + HTML + CSS，Vite 构建（无前端框架 / Tailwind / UI 库） |
| 持久化 | `tauri-plugin-store`（主题 / 字号 / 侧栏 / 最近 / 滚动位置）+ `tauri-plugin-window-state`（窗口大小位置） |

## 架构与项目结构

设计原则是**把「首帧即成品」作为硬约束**：窗口以 `visible: false, create: false` 创建，
前端在离屏 DOM 中完成解析、高亮、公式排版、图片占位与字体预热、滚动位置恢复之后，
才调用唯一的 `show` 点。Rust 侧负责解析、净化与 I/O，前端只负责排版与交互，
两者通过单一 IPC 命令边界通信。

```
.
├── src/                     # 前端（原生 TypeScript，无框架）
│   ├── app/                 # boot / document / math / images / theme / outline / scroll / links …
│   ├── styles/              # app.css / markdown.css
│   ├── main.ts              # 启动编排（唯一入口）
│   └── index.html           # <head> 内联同步脚本：读 __BOOT__ 防主题闪变
├── src-tauri/               # Rust 端
│   ├── src/                 # commands / parser / highlight / sanitize / media / settings / menu / fs …
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── tests/fixtures/          # 手写验收样例（.md + 图片）
├── docs/VERIFICATION.md     # 逐帧 / 交互验收手册
├── SPEC.md                  # 技术 SPEC、设计决策与验收映射
└── package.json
```

## 安全设计

- **解析层**：comrak `render.unsafe_ = false` + `tagfilter`，内联 HTML 全部转义为文本。
- **净化层**：渲染结果统一过 ammonia 白名单；剥离 `style`、`on*`、`srcset`、`form*`；
  URL scheme 仅允许 `http / https / mailto` 与相对路径（`javascript:` 等被移除）。
- **公式**：KaTeX `trust: false`、`throwOnError: false`，`\href` / `\htmlClass` 不生效。
- **CSP**：生产环境 `script-src 'self'`、`default-src 'self'`，仅对 `style-src` 保留
  `'unsafe-inline'` 以支持 KaTeX 行内样式（详见 [SPEC §13](SPEC.md)）。
- **本地图片**：通过 Tauri asset 协议按需授权，只允许文档中真实引用且经 Rust 解析过的路径；
  非法协议（`data:`、`file:` 等）不加载。
- **导航**：链接一律拦截，外链交给系统默认浏览器，应用内不做页面导航。

验收样例见 [tests/fixtures/security.md](tests/fixtures/security.md)。

## 数据与设置

| 内容 | macOS | Windows |
|---|---|---|
| 主题 / 字号 / 侧栏 / 最近文件 / 滚动位置 | `~/Library/Application Support/com.myreader.app/settings.json` | `%APPDATA%\com.myreader.app\settings.json` |
| 窗口大小 / 位置 | `~/Library/Application Support/com.myreader.app/.window-state.json` | `%APPDATA%\com.myreader.app\.window-state.json` |

删除上述文件即可恢复默认设置。

## 已知限制

- 公式识别完全交给 comrak `math_dollars`（Pandoc 启发式）。`价格 $5 和 $6` 这类文本
  若被误判，请用 `\$` 转义；本应用不做启发式修补。
- 远程图片首帧策略：整体最多预加载 800 ms，超时用固定 16:10 占位盒
  （图片随后在盒内 `object-fit: contain` 显示，盒高不变）。
- 本地图片只加载 Markdown 中真实引用且存在的文件（缺失显示 alt 占位）。
- 非法 UTF-8 文件以替换字符宽松解码；超过 32 MB 或含二进制内容的文件拒绝打开并显示错误页。
- 公式与图片远多于 2000 个的极端文档未做懒渲染（当前策略为全部公式在 reveal 前渲染）。
- 仅构建并验证 macOS 本机 arm64 架构，未提供 Universal 二进制；Intel Mac 需自行交叉构建。
- Windows 版本已做代码与打包适配，并提供 GitHub Actions 构建；仍需在 Windows 真机完成人工验收。
- 未做 Apple Developer ID 签名与公证，分发给他人时需处理 Gatekeeper 提示（见下节）。
- Windows 安装包未做代码签名，首次运行可能出现 SmartScreen「未知发布者」提示，选择「更多信息 → 仍要运行」。

## 验收素材

`tests/fixtures/` 下为手写验收样例：

| 文件 | 覆盖内容 |
|---|---|
| [basic.md](tests/fixtures/basic.md) | 标题 / 行内元素 / 嵌套引用 / 嵌套列表 / 分隔线 / 多语言代码块 |
| [elements.md](tests/fixtures/elements.md) | 表格对齐 / 任务列表 / 高亮代码块 / SVG / 缺失图片 / 远程图片 / 脚注 / 删除线 |
| [math.md](tests/fixtures/math.md) | 行内 / 块级 / pmatrix / aligned / cases / 非法公式 / `\$` 转义 / 代码域豁免 |
| [security.md](tests/fixtures/security.md) | 内联 HTML 转义 / `javascript:` 链接剥离 |

手动验收建议：

1. `open -a MyReader.app tests/fixtures/elements.md`：检查各元素与图片占位；
2. 打开 `math.md`：检查公式、矩阵、对齐与错误提示；断网后重开，确认字体本地可用；
3. 编辑任意 md 写入 `<script>alert(1)</script>`、`[x](javascript:alert(1))`、
   `$\href{javascript:alert(1)}{x}$`，确认不执行、不跳转；
4. 退出后重新打开同一篇长文：窗口出现即在记忆的滚动位置；
5. 依次验证拖拽、⌘O / Ctrl+O、Finder / 资源管理器双击三种打开方式；
6. 「显示 → 外观」切换三态并重启验证持久化；切换系统深浅色验证实时跟随；
7. 冷启动录屏逐帧检查：首帧即完整正文、深色系统无白闪、含公式文档无 LaTeX 原文跳变。

逐帧步骤与判定标准见 [docs/VERIFICATION.md](docs/VERIFICATION.md)。

## 分发与 Gatekeeper

本应用没有 Apple Developer ID 证书，只做了 **ad-hoc 完整签名**
（`signingIdentity: "-"`，构建时自动生成 `_CodeSignature`）。通过微信 / 网盘等传给其他 Mac 后，
系统会给文件加隔离属性，首次打开可能提示「已损坏」或「无法验证开发者」——
这是 macOS 的 Gatekeeper 拦截，不是文件损坏。

接收方可任选一种处理（推荐先做第 3 条，再做第一次双击）：

1. 右键 `MyReader.app` → 打开，若出现「仍要打开」则确认；
2. 打开「系统设置 → 隐私与安全性」，滚动到底部点「仍要打开」；
3. 终端执行（先拖入「应用程序」再执行，执行前不要先双击它）：

   ```bash
   sudo xattr -dr com.apple.quarantine /Applications/MyReader.app
   ```

> 彻底免除提示需要 Apple Developer ID 签名 + 公证（notarytool），
> 需要 Apple Developer Program 账号。

## 许可证

本项目基于 [MIT License](LICENSE) 开源，Copyright © 2026 ChanYikLam。

## 致谢

- [Tauri](https://tauri.app)、[comrak](https://github.com/kivikakk/comrak)、
  [syntect](https://github.com/trishume/syntect)、[KaTeX](https://katex.org)、
  [ammonia](https://github.com/rust-ammonia/ammonia)
- 架构与验收细节见 [SPEC.md](SPEC.md) 与 [docs/VERIFICATION.md](docs/VERIFICATION.md)
