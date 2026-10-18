# MyReader — macOS Markdown 阅读器 技术 SPEC

> 状态：**已确认（2026-10-17），进入 M1**
> 版本：v0.1 · 对应需求：用户交付的《项目目标》全文
> 确认记录见 §22。

---

## 1. 目标与范围

用 Rust + Tauri 2 构建一个 **只读** 的 macOS Markdown 阅读器，产品名默认 `MyReader`。
唯一交付物是一个可双击运行、可关联 `.md` 的 `.app`；核心体验是 **打开即成品**：窗口第一次出现时就是完整渲染后的阅读视图，用户永远看不到"未渲染的中间态"。

**范围内**：打开文件（菜单/拖拽/命令行/最近/文件关联）、完整 Markdown 渲染、KaTeX 公式、代码高亮、深浅色三态、目录侧栏、字号、滚动位置与窗口状态记忆、离线可用、安全净化。
**明确不做**：编辑/保存/新建、源码视图、导出、设置面板、同步、插件、自定义 CSS、公式编号/交叉引用、`\(...\)` 定界符、Linux 适配、自动发布。

> 2026-10 变更：新增 Windows 10/11 适配（导航守卫放行 `tauri.localhost`、原生菜单按平台分支、
> 单实例文件关联、Windows 路径规范化、NSIS 打包与 GitHub Actions 构建）。
> Linux 仍不在范围内。

---

## 2. 技术栈（已定，不替换）

| 层 | 选型 | 版本策略 |
|---|---|---|
| 应用壳 | Tauri 2（Rust） | `tauri = "2.12"`，锁文件提交 |
| Markdown | comrak | `0.56`，`default-features = false`，仅开 `syntect-fancy`（纯 Rust 正则，避免 onig C 依赖与 CLI 依赖） |
| 代码高亮 | syntect | `5.3`，`ClassStyle::SpacedPrefixed { prefix: "hl-" }`，class-based HTML |
| 公式排版 | KaTeX | `0.19`，本地 npm 打包 CSS+字体 |
| 前端 | 原生 TypeScript + HTML + CSS + Vite | TS `5.9`、Vite `7`（不使用任何框架/Tailwind/UI 库） |
| 状态持久化 | tauri-plugin-store | `2.5`（Rust 侧读写，前端不直接接触 store） |
| 其它 Tauri 插件 | dialog `2.8`、opener `2.7`、window-state `2.5` | 理由见 §6.1 |

**前端边界（硬约束）**：前端只做三件事——① 把 Rust 返回的 HTML 字符串准备并塞进容器；② 控制 `<html>` 上的主题 CSS 变量/属性；③ 用 KaTeX 渲染公式节点。侧栏、字号、滚动、事件绑定属于这三件事的薄 UI 层，不引入任何框架。

---

## 3. 架构总览

```
┌──────────────────────────── Rust (src-tauri) ────────────────────────────┐
│ main.rs / lib.rs        CLI 参数、窗口创建(__BOOT__ 注入)、事件循环        │
│ commands.rs             #[tauri::command] 边界（唯一 IPC 出口）            │
│ document.rs DocumentPayload: html/headings/images/计时/告警              │
│ parser.rs               comrak Options + AST 后处理(图片/大纲)             │
│ highlight.rs            syntect 单例、class HTML、双主题作用域 CSS        │
│ sanitize.rs             ammonia 白名单净化（渲染后统一执行）              │
│ media.rs                图片相对路径解析、尺寸探测、asset 协议 scope      │
│ theme.rs                ThemeMode 三态、高亮主题映射、原生外观同步        │
│ settings.rs             store schema：主题/字号/滚动/最近/侧栏           │
│ menu.rs                 原生菜单（File/Open Recent/View/Window/Edit）     │
│ error.rs                thiserror AppError → {code,message,detail}        │
│ fs.rs                   读文件：BOM/UTF-8/体积/二进制检测                │
└───────────────────────────────────────────────────────────────────────────┘
        ▲ IPC: invoke(command) / event(open-file)        │
┌───────┴──────────────────── 前端 (src) ───────────────▼───────────────────┐
│ index.html          内联 <head> 同步脚本（读 __BOOT__ 打主题，防闪）      │
│ main.ts             启动编排：bootstrap → 构建 DOM → KaTeX → 字体 → 滚动  │
│                      → sync 原生外观 → reveal（唯一 show 调用点）          │
│ app/document.ts     离屏构建 <article>、图片占位、表格滚动容器             │
│ app/math.ts         KaTeX renderToString（trust:false, throwOnError:false）│
│ app/images.ts       本地尺寸/asset URL、远程预加载、占位 box              │
│ app/theme.ts        三态切换、matchMedia 跟随、CSS 变量                   │
│ app/outline.ts      大纲侧栏、锚点跳转、折叠                               │
│ app/scroll.ts       每文件滚动位置存取、首帧同步恢复                       │
│ app/links.ts        链接拦截：站内锚点滚动 / 外链系统浏览器 / 其余忽略     │
│ app/ipc.ts          invoke 封装与错误归一化                                │
│ app/reveal.ts       show 唯一调用点 + 看门狗                               │
│ styles/*.css        tokens(critical 内联部分) / layout / markdown / app    │
└───────────────────────────────────────────────────────────────────────────┘
```

**依赖方向**：`commands → document/parser/highlight/sanitize/media/settings/menu → error`，单向无环；解析与净化是纯函数（可单测），I/O 集中在 `fs.rs` 与 `settings.rs`。

---

## 4. 目录结构

```
Markdown阅读器/
├── .gitignore
├── SPEC.md
├── README.md                     # M6：开发/构建/验证说明
├── package.json / package-lock.json
├── tsconfig.json / vite.config.ts
├── index.html                    # 内联关键 CSS + 主题同步脚本
├── src/
│   ├── main.ts
│   ├── app/{ipc,document,math,images,theme,outline,scroll,links,settings,reveal}.ts
│   ├── styles/{tokens,layout,markdown,app}.css
│   └── vite-env.d.ts / global.d.ts   # window.__BOOT__ 类型
├── src-tauri/
│   ├── Cargo.toml / Cargo.lock / build.rs
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   ├── icons/                    # 提交（应用资源，不是构建产物）
│   └── src/{main,lib,commands,document,parser,highlight,sanitize,
│            media,theme,settings,menu,error,fs}.rs
└── tests/
    └── fixtures/
        ├── basic.md              # 标题/表格/任务列表/引用/列表/代码块/图片/脚注
        ├── math.md               # 验收 3–4：行内/块级/pmatrix/aligned/非法/转义/代码域
        ├── elements.md           # 全元素覆盖 + 删除线/嵌套引用/hr/链接
        ├── security.md           # <script>/javascript:/onerror/\href 等
        ├── math-many.md          # 200 个公式（性能验收）
        └── images/               # 本地图片夹具（png/jpg/svg，带与不带可探测尺寸）
```

1 MB 性能夹具不入库（验证时临时生成，避免仓库膨胀）；`math.md` 等验收夹具 **手写**，不写生成脚本、不写 demo 页面。

---

## 5. 渲染管线（Rust 侧，纯函数 + 计时）

每次加载文档执行一次，返回 `DocumentPayload`：

1. **读取**（`fs.rs`）：检查后缀（`.md/.markdown/.mdown/.txt`）、体积（> 32 MB 报错）、NUL 字节（二进制报错）；去 UTF-8 BOM；非法 UTF-8 → 宽松解码（替换字符）继续显示（见 §18 假设）。
2. **解析**（`parser.rs`）：comrak `Options`：
   - `extension`: `strikethrough`、`table`、`tasklist`、`footnotes`、`header_ids`、`math_dollars`、`tagfilter`
   - `render.unsafe_ = false` → 内联 HTML **转义为文本**（安全优先；叠加 ammonia 净化）
   - `render.github_pre_lang = false`（输出 `<pre><code class="language-xxx">`）
   - 关闭：`autolink`、`superscript`、`description_lists`、`front_matter_delimiter`（默认按普通 Markdown 渲染）
3. **高亮**（`highlight.rs`）：comrak `SyntectAdapter`（`css_with_class_prefix("hl-")`）+ 单例 `SyntaxSet/ThemeSet`；未知语言 → 自动回退 Plain Text，无报错。
4. **大纲**（`parser.rs`）：comrak AST 遍历标题节点 → `[{level, text, id}]`（id 来自 `header_ids` 扩展）。
5. **图片清单**（`media.rs`）：AST 中所有图片 `src`：
   - `http(s)://` → 远程，前端预加载
   - 相对/绝对本地路径 → 以 md 所在目录解析为绝对路径；`exists`、`width/height`（`imagesize` 只读文件头）
   - 其它协议（`javascript:`、`data:`、`file:` 等）→ 不进入清单，渲染为占位/不加载
6. **渲染 HTML**：`markdown_to_html_with_plugins` (+ syntect adapter) → 统一交给 ammonia 净化（§13）→ 得到最终 `html`。
7. **高亮 CSS**：启动时生成两份 class CSS（`InspiredGitHub` 浅色 / `base16-ocean.dark` 深色），做主题作用域前缀后随 bootstrap 一次性返回；同时从主题 settings 提取代码块前景/背景色作为 CSS 变量值。

计时（parse/highlight/sanitize/IPC 字节数）随 payload 返回，`MYREADER_TIMING=1` 时打到 stderr，用于验收 14。

---

## 6. 依赖清单与理由

### 6.1 Rust（`src-tauri/Cargo.toml`）

| 依赖 | 版本 | 用途 / 新增理由 |
|---|---|---|
| `tauri` | 2.12（feature `protocol-asset`） | GUI 壳；`protocol-asset` 用于本地图片安全加载 |
| `tauri-build` | 2 | Tauri 构建脚本 |
| `tauri-plugin-store` | 2.5 | 需求指定：最近文件/主题/字号/滚动位置持久化（Rust 侧读写） |
| `tauri-plugin-dialog` | 2.8 | 系统文件打开对话框（Cmd+O） |
| `tauri-plugin-opener` | 2.7 | 用系统默认浏览器打开 https 链接 |
| `tauri-plugin-window-state` | 2.5 | 窗口大小/位置记忆；多显示器/缩放/最大化边界已由插件处理，手写易错（备选：自研 store 持久化，见 §20） |
| `comrak` | 0.56 | 需求指定解析器（`syntect-fancy`，不开默认 CLI/onig 特性） |
| `syntect` | 5.3 | 需求指定高亮；只用 `default-themes` + `html` 生成 class CSS |
| `ammonia` | 4.2 | HTML 白名单净化（安全硬要求），基于 html5ever，成熟 |
| `thiserror` | 2 | 需求指定错误类型定义 |
| `serde` / `serde_json` | 1 | IPC payload、`__BOOT__`、设置序列化 |
| `imagesize` | 0.14* | 只读图片文件头拿尺寸，比 `image` 轻两个数量级（*以 Cargo.lock 实际锁定为准） |

不引入：`anyhow`（错误需可序列化，用 thiserror 即可）、`log`/`tracing`（用 `eprintln!` + 计时 env 足够，避免额外依赖）、`image`（太重）。

### 6.2 npm

| 依赖 | 用途 / 理由 |
|---|---|
| `dependencies`: `@tauri-apps/api` ^2.12 | `invoke` / `getCurrentWindow` / 事件监听 / `convertFileSrc` |
| `dependencies`: `katex` ^0.19 | 需求批准的纯渲染库，本地打包 CSS+字体 |
| `devDependencies`: `@tauri-apps/cli` ^2.12 | `npm run tauri build/dev` |
| `devDependencies`: `typescript` ~5.9 | `tsc --noEmit` 类型检查 |
| `devDependencies`: `vite` ^7 | 需求指定构建工具 |

不引入：任何前端框架、Tailwind、UI 库、ESLint/Prettier（验收只要求 `tsc --noEmit`，不为格式工具增加依赖）。

---

## 7. 数学公式：识别规则与边界

### 7.1 识别由 comrak `math_dollars` 扩展完成（Rust 侧）

comrak 0.56 实际规则（已核对源码 `src/parser/inlines.rs`，SPEC 以此为准）：

- 定界符最多两个 `$`：`$...$` → 行内公式，`$$...$$` → 块级（display）。≥3 个 `$` 不开公式。
- 行内公式额外限制（Pandoc 启发式）：
  - 开局 `$` 后不能紧跟空白；结束 `$` 前不能是空白；
  - 结束 `$` 后面不能紧跟数字；
  - 公式内部 `$` 需反斜杠转义（`\$`）；
  - 行内公式可跨行，换行会被规范化为空格（`normalize_code`）。
- 块级 `$$...$$` 无空格限制，可跨行；输出仍是 inline 节点，由 CSS 设为块级居中。
- 代码域豁免：围栏代码块、行内代码中的 `$` 不进入公式解析（comrak 词法层面保证）。
- `\$` 转义 → 输出文本 `$`（普通反斜杠转义，与公式无关）。
- 未闭合/不满足启发式 → 原样作为文本输出。

**Rust → 前端契约**：comrak 输出 `<span data-math-style="inline|display">LaTeX 源码（HTML 转义）</span>`。前端 `querySelectorAll('[data-math-style]')` 取 `textContent` 作为 KaTeX 的 `expression`，用 `katex.renderToString` 渲染后替换该节点。**不做任何启发式"修复"**（`价格 $5 和 $6` 按 comrak 行为处理，用户可用 `\$` 规避）。

### 7.2 前端 KaTeX 配置（固定）

```ts
katex.renderToString(tex, {
  displayMode: style === "display",
  throwOnError: false,
  trust: false,
  strict: "ignore",
  output: "htmlAndMathml",
})
```
- 渲染失败：`throwOnError:false` 让 KaTeX 输出 `katex-error` 节点；前端再包一层 `.math-error`，红色显示**原始 LaTeX + 错误说明**（KaTeX `error-color`），不中断后续渲染（逐节点 try/catch）。
- `trust:false`：`\href`、`\htmlClass`、`\includegraphics` 等不生效。
- 块级公式：容器 `display:block; text-align:center; overflow-x:auto; overflow-y:hidden`（宽公式横向滚动，不撑破布局）。
- 公式颜色：KaTeX 输出不写死颜色，跟随 `color`；CSS 给 `.katex { color: var(--fg) }`，深浅色自动同步。

### 7.3 验收夹具 `tests/fixtures/math.md` 覆盖

行内/块级/多行 `aligned`/`pmatrix`/`bmatrix`/`cases`/分数/根号/积分/希腊字母/上下标/`\text`/非法公式（`\frac{1}` 少参数、`\notacommand{x}`）/`\href{javascript:...}{x}`（不跳转）/`\$5` 转义/围栏代码块内 `$x$`/行内代码 `` `$y$` ``/公式紧邻中文标点。

---

## 8. KaTeX 本地打包方案（离线）

1. `npm i katex@^0.19`，`import "katex/dist/katex.min.css"` 静态引入（Vite 构建产物中为同步 `<link rel="stylesheet">`，随首屏样式一起加载；**绝不在公式渲染后异步 import**）。
2. `katex.min.css` 内字体 URL 由 Vite 重写为 `dist/assets/KaTeX_*.woff2`，随 `.app` 一同打包，无任何 CDN/网络请求。
3. 字体就绪：首帧前 `document.fonts.forEach(f => f.load())` 触发全部 `@font-face`（KaTeX 约 20 个 woff2）加载，再 `await Promise.race([document.fonts.ready, timeout(2000)])`。
4. 验收 5（断网）验证方式：断开网络/Wi-Fi 后冷启动公式文档；并检查构建产物 `dist/assets/` 中字体文件齐全、构建日志无外部 URL。
5. 版本锁定在 `package-lock.json`，字体随版本升级以提交形式可审计。

---

## 9. 首帧即成品：实现方案（核心章节）

### 9.1 窗口创建策略（解决"先白屏/先主题错"的根因）

- `tauri.conf.json` 主窗口：`visible: false`、`create: false`、`backgroundThrottling: "disabled"`、`dragDropEnabled: true`。
- Rust `setup` 中手动创建：`WebviewWindowBuilder::from_config(app.handle(), &window_config)?`
  - `.initialization_script("window.__BOOT__ = <json>;")` —— Tauri 2 保证该脚本在**页面任何脚本之前**执行（WKUserScript documentStart），是主题/字号/滚动同步注入的唯一可靠通道；
  - `<json>` 内容由 Rust 在创建窗口前从 store 读出：`{ themeMode, fontSize, sidebarOpen, initialScroll, windowTheme }`；
  - `.visible(false)`；
  - 创建后立即 `window.show()` 由前端调用（见 9.5），Rust 只负责异步加载文档。
- 不依赖"前端异步读 store 再设主题"（需求明令禁止），也不依赖 localStorage 镜像。
- **CSP**：内联脚本由 Tauri 构建期自动计算 SHA-256 加入 CSP `script-src`（已核对 tauri-codegen 实现）；`<style>` 自动加 nonce。

### 9.2 主题防闪（`index.html` head 内联同步脚本）

```html
<head>
  <script>
    (function () {
      var b = window.__BOOT__ || {};
      var m = b.themeMode || "system";
      var dark = m === "dark" || (m === "system" &&
        window.matchMedia("(prefers-color-scheme: dark)").matches);
      var d = document.documentElement;
      d.dataset.theme = dark ? "dark" : "light";
      d.dataset.themeMode = m;
      d.style.colorScheme = dark ? "dark" : "light";
      if (b.fontSize) d.style.setProperty("--content-font-size", b.fontSize + "px");
    })();
  </script>
  <style>/* 关键布局+两套主题变量内联：html/body 背景、内容宽度、字体栈、侧栏骨架 */</style>
</head>
```
执行顺序：init script（`__BOOT__`）→ 上述内联脚本（设 `data-theme`）→ 内联 CSS（按主题渲染）→ Vite 样式表/模块脚本。窗口此时仍不可见，双保险。

### 9.3 首屏就绪判定（全部满足才 show）

按顺序执行，每项写入 `ReadinessReport`（`MYREADER_TIMING=1` 时输出）：

| # | 条件 | 实现 |
|---|---|---|
| 1 | 主题已应用 | 9.2 内联脚本同步完成（`<html data-theme>`） |
| 2 | 正文 HTML 已入 DOM | `bootstrap` 返回后一次性 `replaceChildren(article)` |
| 3 | **全部**公式已 KaTeX 渲染并插入 | 9.4（比要求的"视口内"更强，见理由） |
| 4 | 字体就绪 | 全部 `@font-face` 显式 load + `document.fonts.ready`（2 s 超时保护） |
| 5 | 视口内图片已加载或已有确定占位 | 本地：Rust 探测宽高 → `width/height` 属性；远程：预加载（≤800 ms）拿自然尺寸，失败则固定 `aspect-ratio` 占位盒；任何图片加载后高度不变 |
| 6 | 滚动位置已同步恢复 | 目标 `scrollTop` 直接赋值；`scroll-behavior: auto`；若被 clamp 则等一帧重试（最多 3 次） |
| 7 | 布局已稳定 | 读取一次 `document.documentElement.scrollHeight` 强制布局 + 双 `requestAnimationFrame`（rAF 有 120 ms 超时兜底，防隐藏窗口不触发） |

**视口定义**：`viewport = [0, window.innerHeight]`（CSS 像素），元素 `getBoundingClientRect()` 与之相交即为"视口内"；滚动恢复完成后重新计算（恢复位置即首帧位置）。
**为什么全部公式都在 show 前渲染**：200 个典型公式实测预期 < 300 ms（KaTeX 单公式约 0.05–0.2 ms），换来 (a) 块级公式高度精确 → 滚动恢复精确、首帧位置准确；(b) 之后无需懒渲染 → 滚动条长度永不跳变（需求 §首帧 5/6 的最强保证）。性能实测不达标时才启用 §20 的降级方案。

### 9.4 启动编排（前端 `main.ts`，时间为小文件预算）

```
0 ms      Tauri 创建隐藏窗口，WebView 加载 index.html
~150 ms   head 内联脚本已设主题/字号；模块脚本执行
~200 ms   invoke('bootstrap') → { themeMode, highlightCss, recent, initialDoc|null }
~250 ms   构建 DOM（离屏 <article>）：注入 HTML → 遍历公式 renderToString（全量）
          → 图片：本地尺寸/asset URL，远程预加载（≤800ms，超时占位）
~400 ms   插入容器 → 字体全量 load + fonts.ready（≤2s 超时）
~500 ms   恢复滚动位置（同步赋值）→ 视口内图片 decode 校验（≤300ms）
~600 ms   双 rAF（≤120ms）→ invoke('sync_window_theme') 设原生标题栏/背景
~650 ms   invoke('reveal_window') → NSWindow.show()    ← 唯一 show 调用点
```
（预算目标：小文件 < 1 s；含 200 公式 < 1.5 s；1 MB 文档 < 1.5 s。M6 用录制实测校准。）

### 9.5 `show` 调用点（唯一）

- 前端 `src/app/reveal.ts` 导出 `revealWindow()`：幂等；先 `invoke('sync_window_theme')`（原生标题栏深浅 + 窗口背景色同步），再 `requestAnimationFrame` 后 `invoke('reveal_window')`（Rust 内调 `window.show()` + `set_focus()`）。
- 除 `revealWindow()` 外，代码中**任何地方不得调用 show**。
- **看门狗**：启动起 4 s 未 reveal（字体/图片/解析异常）→ 强制渲染可用的部分（文档已到则插入，未到则错误/加载页）→ 调用 `revealWindow()`。杜绝"窗口永远不出现"。

### 9.6 错误路径同样 show

读取失败/文件不存在/体积超限/二进制/解析异常：Rust 返回 `AppError{code,message,detail}`，前端在 app shell 内渲染同主题错误页（标题 + 用户可读文案 + "打开文件…"按钮）→ 正常走 9.3/9.5 → show。绝不在 `visible:false` 状态下报错退出。

### 9.7 运行中切换文件（验收 10）

窗口已可见，按"离屏构建 → 一次替换"执行，期间旧内容保持可见：

1. 保存当前文件滚动位置；
2. `invoke('open_path')` → 新 `DocumentPayload`；
3. 在 `position:absolute; left:0; width:<当前内容宽>; visibility:hidden` 的离屏容器中构建新 `<article>`（公式、图片占位全部就绪）；
4. 同一同步任务内：`oldArticle.replaceWith(newArticle)` + `scrollTo(目标位置)` + 更新侧栏/标题/状态；
5. 若解析失败：旧内容保持，右上角/顶部 toast 提示错误（不替换为空白）。

### 9.8 首帧之后（不属于首帧链路的延后项）

本设计在 show 前已完成公式/图片占位/字体，理论上无延后项。show 之后只做：滚动位置持久化（debounce 200 ms）、侧栏可见标题高亮（IntersectionObserver）、事件监听（open-file 菜单/Finder、拖拽）、主题系统监听。禁止在 show 后做会引起滚动条长度变化的事。

### 9.9 首帧验证方法（M1 开始建立，M6 正式验收）

- 环境已具备：`screencapture` 可用、`ffmpeg`（avfoundation）可用。
- 用 `ffmpeg -f avfoundation -framerate 60 -capture_cursor 0` 全屏录制 → 启动 `.app`（带参数/文件关联两种）→ 停止 → 抽帧（`ffmpeg -vf fps=60`）逐帧检查：
  - 第一帧出现 MyReader 窗口时必须是完整正文/主题正确；
  - 深色系统冷启动全程无白色像素帧；
  - 含公式文档第一帧视口内公式已是排版结果；
  - 滚动恢复文档第一帧已处于记忆位置。
- Rust/前端 timing 日志（stderr + `MYREADER_TIMING=1`）交叉验证各阶段耗时。

> 工作约定（2026-10-17）：GUI 启动会干扰用户当前工作，自动验证默认采用无 UI 方式
> （编译/单测/构建产物检查/Node 级 KaTeX 基准等）；需要真实窗口的视觉与首帧验收
> 统一集中到 M6，一次性、经用户同意后执行。

---

## 10. 主题系统

- **三态模型**：`themeMode: "system" | "light" | "dark"`；实际主题 `resolved = system ? matchMedia : mode`。
- **切换入口**：菜单 View → Appearance（跟随系统/浅色/深色，radio 样式）+ 系统外观变化实时跟随（`matchMedia('(prefers-color-scheme: dark)').addEventListener('change')`，仅 system 模式生效）。
- **同步点**（同一 JS 任务内完成，避免中间帧）：`<html data-theme>` → `colorScheme` → CSS 变量整体切换 → Rust `sync_window_theme`（原生标题栏深色/浅色、窗口背景色）。
- **持久化**：手动选择写入 store（`theme.mode`）；重启时由 `__BOOT__` 注入（§9.1）。
- **配色**：浅色近 GitHub 阅读风格（bg `#ffffff`、text `#1f2328`、边框 `#d1d9e0`），深色（bg `#0d1117`、text `#e6edf3`、边框 `#30363d`）；正文/代码/公式/表格边框/引用块背景全部走 CSS 变量。
- **代码高亮**：两份 class CSS 做作用域前缀 `html[data-theme="light"]` / `html[data-theme="dark"]`，纯 CSS 切换，零 JS 时序风险；代码块背景/前景取对应 syntect 主题 settings。
  - 实现注记：Tauri 构建期会往 CSP `style-src` 注入 nonce，按 CSP 规范这会让
    `'unsafe-inline'` 失效、动态 `<style>` 被拦截（实测 `sheet` 为 null）；
    因此高亮 CSS 通过 CSSOM 构造样式表（`adoptedStyleSheets`）注入，不受该限制。
  - syntect 语法集/主题集加载约 100–200ms，启动时后台线程 `prewarm` 与窗口创建并行预热。
- **无过渡动画**：主题切换瞬时完成（不加 `transition`），避免"半深半浅"过渡帧；首帧不激活任何动画。
- 正文字体：系统字体栈 `-apple-system, BlinkMacSystemFont, "PingFang SC", ...`（零 FOUT）；代码字体：`ui-monospace, "SF Mono", Menlo, monospace`。

---

## 11. 持久化与状态（tauri-plugin-store）

`settings.json`（Rust 侧读写，`StoreExt`）：

```jsonc
{
  "theme.mode": "system",                              // 主题偏好
  "typography.fontSize": 17,                           // 13–24，Cmd+/- 步进 1，Cmd+0 复位
  "ui.sidebarOpen": true,                              // 侧栏开合（缺省=按窗口宽度自动）
  "recent": [ { "path": "/abs/a.md", "name": "a.md", "openedAt": 1730000000000 } ], // 去重，最多 10
  "scroll": {
    "/abs/a.md": { "top": 1234, "docHeight": 5600, "viewportWidth": 1000, "updatedAt": 1730000000000 }
  }
}
```
> 按文件路径持久化；`scroll` 超过 200 条按 `updatedAt` LRU 淘汰。窗口大小/位置由
> tauri-plugin-window-state 独立文件保存（多显示器/缩放边界交给插件）。

- 滚动位置：按文件绝对路径存储 `top`（首选）与 `docHeight/viewportWidth`（窗口宽度或文档高度变化过大时按比例回退）。文件数超过 200 条时按 `openedAt` LRU 淘汰（防无限增长）。
- 窗口大小/位置：`tauri-plugin-window-state`（独立文件）；插件在 `on_window_ready` 恢复，兼容手动创建窗口。若与 `create:false` 创建方式冲突，降级为 store 内自存 bounds（手写 clamp 到可见屏幕）。
- 所有设置变更经 Rust 命令落盘（前端不直接用 store JS API）；读只发生在启动 Rust 侧（保证 `__BOOT__` 同步）。

---

## 12. 菜单、打开途径与文件关联

**原生菜单**（Rust `menu.rs`）：
- App 菜单：About / Hide / Quit（`PredefinedMenuItem`）
- File：Open… `Cmd+O`、Open Recent（动态子菜单，最多 10 条，按 `name — 父目录` 展示，含 Clear Recent）`、Close Window `Cmd+W`
- Edit：Copy、Select All（只读阅读器也需复制；无粘贴/剪切）
- View：Appearance ▸ 跟随系统/浅色/深色（radio）；字号 增大 `Cmd+=`/减小 `Cmd+-`/实际大小 `Cmd+0`；Toggle Sidebar `Cmd+B`
- Window：Minimize / Zoom / Bring All to Front（标准）
- Open Recent 变化时重建 File 子菜单（`app.set_menu`）。

**打开文件的 4 条路径**统一收敛到 Rust `open_path(path)` 命令：
1. 菜单/快捷键 → Rust dialog（过滤 `.md/.markdown/.mdown/.txt`）
2. 拖拽 → 前端 `onDragDropEvent` → invoke（多文件取第一个）
3. 命令行：`myreader path/to/file.md`（跳过 `-psn_*` 等参数；相对 cwd 解析）
4. 文件关联：Finder 双击 → `RunEvent::Opened { urls }`（macOS 专用）。若事件在窗口创建后到达，先存入 pending 状态并 emit `open-file`，前端收到后走"运行中切换"路径（§9.7）；启动早期到达则作为初始文档。

**文件关联配置**：`bundle.fileAssociations`：`ext: ["md","markdown","mdown","txt"]`、`name`、`description`、`role: "Viewer"`（只读）、`mimeType: "text/markdown"`；`bundle.macOS.minimumSystemVersion: "11.0"`；`targets: ["app"]`。M6 用 `open -a <app> file.md` 与 Finder 双击实机验证。

**链接策略**（`app/links.ts`）：内容区 click 拦截：
- `#锚点` → 站内滚动（脚注、大纲）
- `http/https/mailto` → `invoke('open_external')`（tauri-plugin-opener）
- 其它/非法 → preventDefault 忽略
- 应用内导航一律 preventDefault（`will-navigate` 无需额外处理，Tauri 不开放远程导航）。

---

## 13. 安全设计

1. **解析层**：comrak `render.unsafe_ = false` → 内联 HTML 一律转义为文本（`<script>` 显示为文字）；`tagfilter` 扩展叠加。
2. **净化层**：渲染完成后统一过 ammonia：
   - 标签白名单（显式列出）：`a, blockquote, br, code, del, div, em, h1–h6, hr, img, input, kbd, li, ol, p, pre, s, section, span, strong, sub, sup, table, tbody, td, th, thead, tr, ul, sup, section`（含脚注所需）；其余标签剥离。
   - 属性：`class`、`id`、`title` 作为 generic attribute；`data-*`/`aria-*` 前缀允许（`data-math-style`、`data-footnotes`、`aria-describedby`）；`img` 的 `src/alt/width/height/loading/decoding`；`a` 的 `href`；`input` 的 `type/checked/disabled`；`ol` 的 `start`；`td/th` 的 `colspan/rowspan/align`。
   - 明确**不允许** `style`、`on*`、`srcset`、`form*`。
   - URL scheme 白名单：`http/https/mailto`（相对 URL 保留，由前端解析为本地图片）；其余 scheme 属性剥离。
   - 净化后仍保留 `class`（syntect/highlight 需要）与 `id`（锚点需要）。
3. **KaTeX**：`trust: false`、`throwOnError: false`、`strict: "ignore"`（`\href`/`\htmlClass` 不生效）。
4. **CSP**（生产）：
   `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' asset: http: https: data:; font-src 'self' data:; connect-src ipc: http://ipc.localhost`
   （`style-src 'unsafe-inline'` 因 KaTeX 行内 style 属性；dev 用 `devCsp` 放宽 HMR ws + 内联脚本）。
   实现注记：Tauri 构建期会往 `style-src` 注入 nonce，按 CSP 规范这会让 `'unsafe-inline'`
   失效，从而屏蔽 KaTeX 公式排版所必需的行内 `style` 属性。因此对 `style-src`
   关闭 CSP 自动修改（`dangerousDisableAssetCspModification: ["style-src"]`）；
   `script-src` 仍由 Tauri 注入 hash，用户内容已 sanitize 且禁止 `style` 属性，风险可控。
5. **本地图片**：经 Tauri asset 协议加载（`app.security.assetProtocol.enable = true`），加载前 Rust 对每张图片 `allow_file(绝对路径)`；只允许 HTML 中真实出现、且被 Rust 解析过的图片路径，不开放任意目录。
6. **导航**：链接一律 preventDefault，不提供应用内导航入口。
7. **验收 12 三例**：`<script>alert(1)</script>`（转义为文本，不执行）、`[x](javascript:alert(1))`（ammonia 剥离、前端拦截）、`$\href{javascript:alert(1)}{x}$`（trust:false，公式内不跳转）。

---

## 14. 性能设计与验证

| 指标 | 目标 | 手段 | 测量 |
|---|---|---|---|
| 双击到首帧（<100KB） | < 1 s | 隐藏窗口 + 全预渲染流水线；comrak/syntect 单例预热 | ffmpeg 抽帧 + timing 日志 |
| 含公式小文件 | < 1 s | 全部公式 show 前渲染（预期 <300ms） | 同上 |
| 1 MB md 首屏 | < 1.5 s | `spawn_blocking` 解析、HTML 一次 IPC、避免二次字符串拷贝；DOM 一次插入 | timing + 录屏 |
| 滚动流畅 | 不掉帧 | 无阴影/无 `backdrop-filter`、图片 `content-visibility:auto`（仅视口外图片区域）、避免超大 DOM 重排 | 录屏抽帧主观检查 |
| 200 公式 | 视口就绪 <1.5 s，视口外 2 s 内补齐 | show 前全量完成（0 s 补齐） | timing |
| 滚动条不跳 | 无长度变化 | 全量公式预渲染 + 图片确定性占位 + `overflow-anchor:none` | 录屏/手动滚动检查 |

---

## 15. 测试策略

- **Rust 单元测试**（`cargo test`，内联 `#[cfg(test)]`）：
  - `parser`：标题/表格/任务列表/脚注/删除线/嵌套引用/代码块 html 结构断言；数学公式边界（§7 全部规则）、`\$` 转义、代码域豁免、非法公式节点保留；大纲提取。
  - `sanitize`：script/on*/javascript:/href 剥离、`data-math-style`/`class`/`id` 保留、禁止标签/属性表。
  - `highlight`：高亮 HTML 为 class-based、未知语言无 panic、CSS 作用域前缀正确。
  - `media`：相对路径解析、`..` 路径、URL 编码文件名、尺寸探测（png/jpg/svg 无尺寸）。
  - `fs`：BOM 去除、非法 UTF-8 宽松、二进制/超大拒绝。
- **夹具驱动**：`tests/fixtures/*.md` 为手写验收素材（不生成假数据）。
- **前端**：`tsc --noEmit` 强类型；`npm run build` 通过；无自动化 DOM 测试框架（不引入额外依赖，前端逻辑保持薄）。
- **手动验收**：按需求 1–18 条逐条记录（M6 输出验收记录表，含录屏证据描述）。
- **提交门禁**：每个提交前 `cargo check`、相关改动时 `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check`、`tsc --noEmit`、`vite build` 全部通过。历史可回滚：每个 commit 独立可编译。

---

## 16. 提交计划（预计 6 个里程碑 + 1 个 SPEC 提交，约 56 个提交）

> 提交粒度 = 一个可编译、可验证的逻辑单元；每完成一个里程碑：工作区干净 + `git log --oneline <上一tag>..HEAD` 摘要 + tag `milestone/mN`。

### M0 · SPEC（1 提交，已完成/待确认）
1. `docs(spec): 初始化仓库并编写 SPEC`（`.gitignore` + `SPEC.md`）

### M1 · 骨架 + 打开文件 + 基础渲染 + 延迟 show（约 13 提交）
1. `chore(build): 初始化 Tauri 2 + Vite + TypeScript 工程骨架`（含最小可编译窗口）
2. `feat(fs): 实现文档读取与编码/体积校验`
3. `feat(parser): 接入 comrak 输出基础 Markdown HTML`
4. `feat(parser): 渲染结果经 ammonia 白名单净化`
5. `feat(ui): 建立阅读视图布局与基础主题样式`
6. `feat(ui): 渲染文档 HTML 与启动错误页`
7. `feat(ui): 实现延迟显示窗口的首屏就绪机制`
8. `fix(ui): 启动失败时同样显示窗口与错误信息`
9. `feat(menu): 实现 File 菜单与 Cmd+O 打开对话框`
10. `feat(fs): 支持拖拽打开与命令行参数`
11. `feat(ui): 链接点击交给系统浏览器并拦截非法协议`
12. `test(parser): 添加基础渲染夹具与解析测试`
13. `chore(milestone): M1 里程碑验证与 tag`
（M1 第 1 提交前先做隐藏窗口探针实验：rAF/字体加载/合成行为，结果记入 commit body；若不通过启用 §20 备选方案。）

### M2 · 代码高亮 + 表格 + 图片（约 9 提交）
1. `feat(highlight): 接入 syntect 生成 class-based 高亮 HTML`
2. `feat(highlight): 生成明暗两套作用域高亮 CSS`
3. `feat(parser): 开启表格/任务列表/脚注/删除线扩展`
4. `feat(ui): 表格横向滚动与引用块/任务列表样式`
5. `feat(fs): 解析本地图片相对路径并探测原始尺寸`
6. `feat(ui): 图片确定性占位与首屏预加载`
7. `feat(ui): 远程图片加载与失败占位`
8. `fix(highlight): 未知语言代码块降级为纯文本`
9. `test(parser): 表格/脚注/高亮夹具与断言`
+ tag `milestone/m2`

### M3 · 数学公式（约 10 提交）
1. `feat(math): comrak 开启 math_dollars 并输出公式占位节点`
2. `build(web): 安装 KaTeX 并本地打包 CSS 与字体`
3. `feat(math): 前端用 KaTeX 渲染公式节点`
4. `feat(math): 公式渲染失败显示原文与错误提示`
5. `feat(math): 首帧前全量渲染公式保证高度稳定`
6. `test(parser): 覆盖公式识别边界规则`
7. `test(math): 添加 tests/fixtures/math.md 验收样例`
8. `style(math): 块级公式居中与主题颜色适配`
9. `perf(math): 公式渲染耗时计时与阈值保护`
10. `fix(math): 代码域中的 $ 不参与公式解析`
+ tag `milestone/m3`

### M4 · 深浅色（约 7 提交）
1. `feat(theme): 建立三态主题模型与 CSS 变量体系`
2. `feat(theme): 内联同步脚本实现首帧主题无闪`
3. `feat(theme): 菜单切换跟随系统/浅色/深色`
4. `feat(theme): 系统外观变化实时跟随`
5. `feat(theme): 代码高亮与公式颜色随主题切换`
6. `feat(theme): 主题偏好持久化并在启动注入`
7. `feat(theme): 原生标题栏/窗口外观同步`
+ tag `milestone/m4`

### M5 · 目录 / 字号 / 滚动位置 / 最近 / 窗口记忆（约 9 提交）
1. `feat(parser): 提取标题大纲并生成锚点`
2. `feat(ui): 实现目录侧栏与点击跳转`
3. `feat(ui): 侧栏折叠与响应式布局`
4. `feat(ui): 字号调节快捷键与持久化`
5. `feat(state): 记录每个文件的滚动位置`
6. `feat(state): 首帧同步恢复滚动位置`
7. `feat(state): 窗口大小与位置记忆`
8. `feat(state): 最近打开文件列表`
9. `feat(menu): File → Open Recent 动态菜单与清空`
+ tag `milestone/m5`

### M6 · 文件关联 + 打包 + 逐帧自测（约 8 提交）
1. `feat(fs): 支持 macOS 文件关联与 Finder 打开事件`
2. `feat(ui): 运行中切换文件离屏构建再原子替换`
3. `build(bundle): 配置 Info.plist、图标与 .app 打包`
4. `perf(parser): 1MB 文档解析与高亮开销优化`
5. `fix(ui): 修复打包后 CSP 与资源路径问题`
6. `docs: 添加 README、开发说明与验收记录`
7. `test: 补充安全与全元素验收夹具`
8. `chore(release): M6 逐帧验证与最终打包`
+ tag `milestone/m6`

> 实际提交数可按"逻辑单元"微调（合并/拆分），但不会压成里程碑巨型提交，也不会出现 typo 级碎提交。

---

## 17. 本机环境适配（本会话构建约束，不影响用户正常使用）

- 本机 `~/.cargo`、`~/.npm` 不可写（沙箱 + 既有 root 属主缓存）：
  - 构建时使用工作区内缓存：`CARGO_HOME=$PWD/.cargo-home`、`npm_config_cache=$PWD/.npm-cache`；
  - 两者加入 `.gitignore`；
  - 用户正常终端里 `npm run tauri build` 不受影响（`node_modules` 已安装、cargo 使用其默认 `~/.cargo`）。
- 工具链：Rust 1.96 / Node 22 / Apple CLT（无完整 Xcode，Tauri 构建可用）；目标架构 `aarch64-apple-darwin`（不做 universal 二进制）。
- 构建产物路径：`src-tauri/target/release/bundle/macos/MyReader.app`（`.app` 已被忽略）。
- 每次提交前门禁脚本（手动执行，不引入 CI）：
  ```
  cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
  cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
  npm run typecheck   # tsc --noEmit
  npm run build       # vite build
  ```

---

## 18. 假设（需求未写明处的默认决定，请确认或纠正）

| # | 问题 | 默认假设 |
|---|---|---|
| A1 | 应用名 / Bundle ID | 产品名 `MyReader`，`identifier = com.myreader.app`，二进制 `myreader` |
| A2 | 无参数启动（无文件、无最近记录要求） | 显示同主题的欢迎空状态（应用名 + "打开文件 Cmd+O" + 最近文件列表），不自动打开上次文件 |
| A3 | 目录侧栏默认状态与层级 | 大纲收 h1–h3；窗口 ≥ 900px 默认展开，< 900px 默认收起；用户手动开合持久化（`ui.sidebarOpen` 优先于宽度推断） |
| A4 | 远程图片首屏策略 | 先预加载（整体上限 800 ms）拿自然尺寸；超时/失败用固定 `aspect-ratio: 16/10` 占位盒（图片 `object-fit: contain`，加载后高度不变） |
| A5 | 本地图片合法性 | 允许 md 所在目录及其相对 `..` 解析出的真实文件（只作为图片加载）；不存在的图片显示同尺寸占位 |
| A6 | 编码 | 去 UTF-8 BOM；非法 UTF-8 宽松解码（替换字符）继续显示 |
| A7 | 文件体积上限 | 32 MB；超过显示错误页（防 OOM 卡死） |
| A8 | YAML front matter | 不特殊处理，按普通 Markdown 渲染（不在需求内） |
| A9 | 默认字号 | 17 px；范围 13–24；Cmd+=/Cmd+- 步进 1；Cmd+0 复位 17 |
| A10 | 复制 | 提供 Edit 菜单 Copy/Select All（只读复制），不做任何编辑能力 |
| A11 | 主题配色 | 浅色/深色自选接近 GitHub 阅读风格的配色（§10），高亮主题 InspiredGitHub / base16-ocean.dark |
| A12 | 最近列表 | 最多 10 条，去重按最近打开排序；含 Clear Recent；文件不存在时打开显示错误页并从列表保留（不自动清理） |
| A13 | 单实例 | 不引入单实例插件；Finder/菜单已覆盖日常场景（同时开两个实例不阻止） |
| A14 | 窗口标题 | 显示当前文件名；未打开文件时显示 `MyReader` |
| A15 | 大纲层级显示 | 侧栏仅收 h1–h3（层级再深的标题不出现在大纲） |
| A16 | 启动看门狗 | 4 s 内未就绪则强制显示当前可用状态（不无限卡隐藏窗口） |
| A17 | `.txt` 文件 | 按 Markdown 渲染（同为文本阅读，不进编辑器） |

---

## 19. 风险与备选方案

| 风险 | 触发条件 | 备选方案 |
|---|---|---|
| 隐藏窗口下 WKWebView 不触发 rAF/字体加载，show 后仍可能空白一帧 | M1 探针实验 | 备选 A：保留 `visible:false`，用 `backgroundThrottling:disabled` + 显式字体 load + `setTimeout` 兜底（本方案已有）；备选 B：`NSWindow.setAlphaValue(0)` → show → 首帧后 alpha 1（objc2，零白闪但实现稍重）；备选 C：窗口背景色=主题色遮罩，show 后同帧替换内容（会有一帧纯色，验收 6 可能不过，仅作最后兜底） |
| `create:false` + `from_config` 与 window-state 插件不兼容 | M5 | 手写窗口 bounds 持久化到 store + 创建窗口前 clamp 到可见屏幕（+80 行） |
| 1 MB 文档 HTML IPC 传输/插入超预算 | M6 | 解析结果直接写临时文件 + 前端 `fetch` 本地协议读取；或分块 IPC（先首屏块后剩余块） |
| 全部公式预渲染在超多公式文档上过慢 | 实测 >1.5 s | 视口优先：视口内同步渲染；视口外先用"离屏渲染→测高→写死高度"的占位批次，show 后填充（scrollbar 不跳） |
| comrak/syntect 版本行为变化 | 升级时 | Cargo.lock 锁定 + 单元测试回归 |
| 菜单/窗口/文件关联的 Tauri 平台细节偏差 | 实现期 | 以实测为准微调实现细节，不改变验收目标 |

---

## 20. 验收映射（需求 1–18 → 实现/证据）

| 验收 | 对应章节/证据 |
|---|---|
| 1 打包 .app | §12、M6；`src-tauri/target/release/bundle/macos/MyReader.app` |
| 2 元素渲染正确 | §5、§7、§15；`tests/fixtures/basic.md`、`elements.md` |
| 3 公式排版 | §7、§8；`tests/fixtures/math.md` |
| 4 代码域/转义/非法公式 | §7.1/§7.2；`math.md` 边界段 + 单测 |
| 5 断网公式 | §8 |
| 6 冷启动首帧 | §9 全章；M6 ffmpeg 抽帧记录 |
| 7 深色冷启动无白闪 | §9.2/§9.3；M4/M6 录屏 |
| 8 公式首帧已排版 | §9.3 第 3 项；M6 录屏 |
| 9 滚动位置首帧恢复 | §9.3 第 6 项、§11；M5/M6 |
| 10 运行中切换无空白 | §9.7；M6 |
| 11 主题切换/持久化 | §10、§11；M4 |
| 12 注入攻击不执行 | §13；`tests/fixtures/security.md` |
| 13 外链浏览器打开 | §12、§13 |
| 14 性能指标 | §14；timing 日志 + 录屏 |
| 15 Finder 双击 | §12；M6 `open -a`/Finder 实测 |
| 16–18 Git 历史 | §16 + 每个里程碑的 log/status 摘要 |

---

## 21. 需求确认（2026-10-17 已确认）

1. **应用名 / Bundle ID**：`MyReader` / `com.myreader.app`（已确认）
2. **无文件启动**：欢迎页 + 最近打开列表（已确认）
3. **侧栏默认**：宽窗口（≥900px）展开、窄窗口收起，大纲收 h1–h3，记住用户选择（已确认）
4. **远程图片首屏**：先预加载、整体最多等 800 ms，超时用固定 16:10 占位盒（已确认）
5. **§18 其余假设（A1–A17）**：全部按默认执行（已确认）

> 确认结论：按本 SPEC 进入 M1，不再等待额外确认。

---

## 22. 确认记录

| 日期 | 事项 | 结论 |
|---|---|---|
| 2026-10-17 | SPEC 评审 | 用户确认 §18 全部默认假设（A1–A17）与 §21 五项选择，同意按 §16 提交计划开始 M1 |


---

## 25. M6 验收记录（2026-10-18）

### 25.1 首帧/视觉（真实窗口，录屏抽帧 + 窗口截图）

| 场景 | 结果 |
|---|---|
| 浅色冷启动（elements.md） | 窗口首次出现的那一帧即为完整正文；前帧为桌面，无空白/半成品窗口帧 |
| 深色冷启动（持久化 dark） | 首帧即完整深色内容，无白色/浅色窗口过渡帧 |
| 含公式冷启动（math.md） | 首帧公式已排版；pmatrix/aligned/cases 全部正确（含 `$$` 块内独立 `=` 行） |
| 滚动位置恢复（1.7MB 文档） | 首帧即记忆位置（第 296 节），无滚动动画 |
| 字号 20 + 侧栏收起 冷启动 | 首帧即 20px 字号、侧栏收起（悬浮按钮可见） |
| 欢迎页（含最近列表） | 显示最近打开条目，点击可打开 |
| 缺失图片 | 固定 16:10 占位框 + 居中 alt 文案（修复了损坏图标 alt 溢出问题） |
| 安全样例 | `<script>`/事件属性显示为文本；`javascript:` 链接被剥离为纯文本样式 |

> 首帧缺陷修复：`tauri-plugin-window-state` 默认恢复 visible 并在创建时 `show()`，
> 绕过 reveal 门禁造成空白窗口约 0.7s。已改为 `StateFlags::all() & !VISIBLE`，
> 窗口可见性只由前端 reveal 门禁控制。

### 25.2 性能（安装版 /Applications/MyReader.app，open → 完整首帧可见，2 次取范围）

| 文档 | 首帧耗时 |
|---|---|
| 欢迎页（无文档） | 0.58–1.63s（首次冷启动 1.63s） |
| math.md（含公式夹具） | 0.73s |
| 200 公式文档 | 0.75–0.79s |
| **1MB 真实分布文档** | **1.33–1.45s（验收 14 达标）** |
| 1.7MB 压力文档（3136 标题级/735 公式/1027 代码块） | 2.85s |

优化记录：
- 隐藏窗口下 WebKit 节流字体加载回调（6 个字面 2.86s）→ JS 启动最早期并行 warm
  全部字面，与 Rust 解析/IPC/DOM 渲染重叠（字体等待 2.86s → 0.40s）；
- 隐藏窗口 rAF 不触发 → 字体/公式就绪均显式等待；
- 目录大纲改为 reveal 后一次性缓存标题偏移 + 滚动二分查找；
- 剥离 comrak 标题 permalink 锚点（只读阅读器用不到，减少 HTML/DOM 体积）；
- 看门狗 4s → 15s：首帧完整性优先，避免大文档被提前显示为空白窗口。

### 25.3 交付物

- 安装位置：`/Applications/MyReader.app`（已注册 LaunchServices，Finder 双击 .md 可用）；
- DMG：`src-tauri/target/release/bundle/dmg/MyReader_0.1.0_aarch64.dmg`（4.3 MB）；
- 右上角 ◐ 按钮：循环 跟随系统 → 浅色 → 深色；⌘B/⌘=/⌘-/⌘0 快捷键可用。

### 25.4 最终构建（安装版）复核说明

- 复测环境：`/Applications/MyReader.app`（13:08 构建，含主题按钮/性能优化/占位符修复）。
- 深色冷启动与含公式冷启动：窗口截图确认内容正确（深色配色、pmatrix/aligned/cases 已排版、
  右上角主题按钮图标随模式变化）；性能计时 1MB 首帧 1.33–1.45s 在本构建上实测。
- 滚动恢复首帧的逐帧录屏证据来自修复 window-state 可见性后的构建（滚动逻辑此后未改动，
  仅追加字体预热/占位符/按钮）；本次复测因用户正在使用电脑，录屏采集到其他 Space，
  无法在当前 Space 拍到窗口首帧——**该条以修复后构建的逐帧证据为准**，如需可在空闲时复测。
- 说明：屏幕录制只能拍到当前 Space；窗口截图（`screencapture -l`）可拍任意 Space 的窗口，
  但窗口在非活动 Space 时 WebKit 不会提交绘制（截图为未绘制的启动占位），
  这不代表应用首帧缺陷。
- 仍建议用户手动确认的验收项：Finder 双击 .md（15）、系统外观实时切换（11）、
  点击外链（13）、断网首次打开公式文档（5）、菜单/按钮点击交互。
