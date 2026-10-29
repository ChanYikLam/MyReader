#!/usr/bin/env node
/**
 * Rust 构建产物体积预算检查。
 *
 * - 预算来源：仓库根目录 `size-budget.json`（每项上限可写成 "1.5 GiB" 这类可读值，
 *   也可以按平台给不同上限，例如 `{ "darwin": "1.5 GiB", "win32": "3 GiB" }`）；
 * - 被测产物：开发构建目录（target/debug）与 release 可执行文件 / macOS 分发包 /
 *   Windows 安装包；当前平台不适用的产物自动跳过；
 * - 未构建的产物默认跳过（开发机上只构建了 dev 或只构建了某个 bundle 都很常见），
 *   只要存在的产物都在上限内即通过；
 * - 任一产物超出上限时以退出码 1 失败，便于本地门禁与 CI 直接引用。
 *
 * 无第三方依赖，macOS / Windows / Linux 通用；--root、--budget 供测试夹具使用。
 */
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, "..");
const PLATFORM = process.platform;

/** 体积单位（B 按字节，KiB/MiB/GiB 按 1024 进制；KB/MB/GB 按 1000 进制）。 */
const UNITS = {
  b: 1,
  kb: 1e3,
  kib: 1024,
  mb: 1e6,
  mib: 1024 ** 2,
  gb: 1e9,
  gib: 1024 ** 3,
};

/** 参与检查的构建产物定义；limits 里的 key 与 id 一一对应。 */
const ARTIFACTS = [
  {
    id: "dev.debugTargetDir",
    profile: "dev",
    label: "开发构建目录",
    kind: "dir",
    path: "src-tauri/target/debug",
    platforms: ["darwin", "win32", "linux"],
  },
  {
    id: "release.binary",
    profile: "release",
    label: "release 可执行文件",
    kind: "file",
    candidates: [
      "src-tauri/target/release/myreader",
      "src-tauri/target/release/myreader.exe",
    ],
    platforms: ["darwin", "win32", "linux"],
  },
  {
    id: "release.macosApp",
    profile: "release",
    label: "macOS 应用包（.app）",
    kind: "dir",
    globDir: "src-tauri/target/release/bundle/macos",
    suffix: ".app",
    platforms: ["darwin"],
  },
  {
    id: "release.macosDmg",
    profile: "release",
    label: "macOS 磁盘映像（.dmg）",
    kind: "file",
    globDir: "src-tauri/target/release/bundle/dmg",
    suffix: ".dmg",
    platforms: ["darwin"],
  },
  {
    id: "release.windowsNsis",
    profile: "release",
    label: "Windows 安装包（NSIS）",
    kind: "file",
    globDir: "src-tauri/target/release/bundle/nsis",
    suffix: ".exe",
    platforms: ["win32"],
  },
  {
    id: "release.windowsMsi",
    profile: "release",
    label: "Windows 安装包（MSI）",
    kind: "file",
    globDir: "src-tauri/target/release/bundle/msi",
    suffix: ".msi",
    platforms: ["win32"],
  },
];

const HELP = `Rust 构建产物体积预算检查

用法：
  node scripts/check-size.mjs [选项]

选项：
  --profile <dev|release|all>  要检查的构建档，默认 all
  --root <dir>                 仓库根目录（默认脚本所在仓库）
  --budget <file>              预算配置文件（默认 <root>/size-budget.json；
                               相对路径按当前工作目录解析）
  --json                       以 JSON 输出结果（便于 CI 消费）
  -h, --help                   显示本帮助

退出码：
  0  通过（含「产物未构建」的跳过项）
  1  存在超出体积上限的产物
  2  参数或预算配置错误

示例：
  npm run size                  # 本地校验当前已有的构建产物
  node scripts/check-size.mjs --profile dev
  node scripts/check-size.mjs --profile release --json
`;

/** 解析命令行参数。 */
function parseArgs(argv) {
  const args = { profile: "all", root: REPO_ROOT, budget: null, json: false, help: false };
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (token === "-h" || token === "--help") {
      args.help = true;
    } else if (token === "--json") {
      args.json = true;
    } else if (token === "--profile" || token === "--root" || token === "--budget") {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith("-")) {
        throw new UsageError(`选项 ${token} 缺少取值`);
      }
      i += 1;
      if (token === "--profile") {
        if (!["dev", "release", "all"].includes(value)) {
          throw new UsageError(`--profile 只接受 dev / release / all，收到：${value}`);
        }
        args.profile = value;
      } else if (token === "--root") {
        args.root = resolve(value);
      } else {
        args.budget = resolve(value);
      }
    } else {
      throw new UsageError(`未知参数：${token}（--help 查看用法）`);
    }
  }
  if (args.budget === null) {
    args.budget = join(args.root, "size-budget.json");
  }
  return args;
}

class UsageError extends Error {}

/** 把 "1.5 GiB" / 8192 / {"darwin": "1.5 GiB"} 解析成字节数。 */
function parseLimit(raw, where) {
  if (typeof raw === "number") {
    if (Number.isFinite(raw) && raw > 0) {
      return raw;
    }
    throw new UsageError(`体积上限无效：${where} = ${raw}（应为正数字节数）`);
  }
  if (typeof raw !== "string") {
    throw new UsageError(`体积上限无效：${where} = ${JSON.stringify(raw)}（应形如 "1.5 GiB"）`);
  }
  const matched = /^([0-9]+(?:\.[0-9]+)?)\s*(b|kb|kib|mb|mib|gb|gib)?$/i.exec(raw.trim());
  if (!matched) {
    throw new UsageError(
      `体积上限无效：${where} = ${JSON.stringify(raw)}（应形如 "1.5 GiB" / "512 MiB" / "20480 KB"）`,
    );
  }
  const amount = Number.parseFloat(matched[1]);
  const unit = (matched[2] ?? "b").toLowerCase();
  const bytes = Math.round(amount * UNITS[unit]);
  if (!Number.isFinite(bytes) || bytes <= 0) {
    throw new UsageError(`体积上限必须大于 0：${where} = ${JSON.stringify(raw)}`);
  }
  return bytes;
}

/** 解析当前平台适用的上限；返回 null 表示该产物在当前平台显式豁免。 */
function parseLimitForPlatform(raw, id, platform) {
  if (raw !== null && typeof raw === "object" && !Array.isArray(raw)) {
    if (Object.hasOwn(raw, platform)) {
      // 平台 key 显式写成 null 表示该平台不做限制。
      return raw[platform] === null ? null : parseLimit(raw[platform], `${id}（${platform}）`);
    }
    if (Object.hasOwn(raw, "default")) {
      return parseLimit(raw.default, id);
    }
    const configured = Object.keys(raw).join(" / ");
    throw new UsageError(
      `体积上限 ${id} 缺少当前平台（${platform}）的配置，也没有 default；` +
        `已配置：${configured || "（空）"}。请补 ${platform} 或 default，或用 null 显式豁免。`,
    );
  }
  return parseLimit(raw, id);
}

/** 读取并校验预算配置。 */
function loadBudget(file) {
  let text;
  try {
    // 兼容 Windows 编辑器写入的 UTF-8 BOM。
    text = readFileSync(file, "utf8").replace(/^\uFEFF/, "");
  } catch (error) {
    throw new UsageError(`读取预算配置失败：${file}（${error.message}）`);
  }
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    throw new UsageError(`预算配置不是合法 JSON：${file}（${error.message}）`);
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new UsageError(`预算配置应为对象：${file}`);
  }
  if (parsed.version !== 1) {
    throw new UsageError(`预算配置 version 必须为 1：${file}（当前 ${JSON.stringify(parsed.version)}）`);
  }
  if (parsed.limits === null || typeof parsed.limits !== "object" || Array.isArray(parsed.limits)) {
    throw new UsageError(`预算配置缺少 limits 对象：${file}`);
  }
  return parsed.limits;
}

/** 递归统计路径逻辑体积（字节）。
 *
 * 规则：跟随符号链接（兼容「target 放在外置盘/被软链」的情况）；按 (dev, inode)
 * 去重，避免软链环路与硬链接重复计数；不可读目录按 0 计入。
 */
function pathSize(target, seen = new Set()) {
  let stat;
  try {
    stat = statSync(target);
  } catch {
    return 0;
  }
  if (stat.isDirectory()) {
    const key = `d:${stat.dev}:${stat.ino}`;
    if (seen.has(key)) {
      return 0;
    }
    seen.add(key);
    let total = 0;
    let entries;
    try {
      entries = readdirSync(target, { withFileTypes: true });
    } catch {
      return 0;
    }
    for (const entry of entries) {
      total += pathSize(join(target, entry.name), seen);
    }
    return total;
  }
  if (stat.isFile()) {
    const key = `f:${stat.dev}:${stat.ino}`;
    if (seen.has(key)) {
      return 0;
    }
    seen.add(key);
    return stat.size;
  }
  return 0;
}

/** 把字节数格式化成可读文本（按数量级自动选单位）。 */
function formatBytes(bytes) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let index = -1;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  const text = value.toFixed(value >= 100 ? 0 : value >= 10 ? 1 : 2).replace(/\.0+$/, "");
  return `${text} ${units[index]}`;
}

/** 估算终端显示宽度：CJK 与全角字符按 2 列计。 */
function displayWidth(text) {
  let width = 0;
  for (const char of String(text)) {
    width += char.codePointAt(0) > 0x2e7f ? 2 : 1;
  }
  return width;
}

/** 按显示宽度右侧补空格。 */
function padDisplay(text, width) {
  return String(text) + " ".repeat(Math.max(0, width - displayWidth(text)));
}

/** 相对仓库根显示路径，统一使用 /。 */
function displayPath(root, absolutePath) {
  const rel = relative(root, absolutePath) || ".";
  return rel.split(sep).join("/");
}

/** 找到某个产物定义在磁盘上实际存在的路径列表。 */
function resolveArtifactPaths(root, artifact) {
  const toAbsolute = (relativePath) => resolve(root, ...relativePath.split("/"));
  if (artifact.path) {
    const single = toAbsolute(artifact.path);
    return existsSync(single) ? [single] : [];
  }
  if (artifact.candidates) {
    return artifact.candidates.map(toAbsolute).filter((candidate) => existsSync(candidate));
  }
  const dir = toAbsolute(artifact.globDir);
  let names;
  try {
    names = readdirSync(dir);
  } catch {
    return [];
  }
  const suffix = artifact.suffix.toLowerCase();
  return names
    .filter((name) => name.toLowerCase().endsWith(suffix))
    .sort()
    .map((name) => join(dir, name));
}

/** 执行检查，返回可序列化结果。 */
function runChecks(options) {
  const budget = loadBudget(options.budget);
  const knownIds = new Set(ARTIFACTS.map((artifact) => artifact.id));
  const unknownIds = Object.keys(budget).filter((id) => !knownIds.has(id));
  if (unknownIds.length > 0) {
    throw new UsageError(
      `size-budget.json 中存在无法识别的产物 id：${unknownIds.join(", ")}；` +
        `可用的 id：${[...knownIds].join(", ")}`,
    );
  }

  const profiles = options.profile === "all" ? ["dev", "release"] : [options.profile];
  const results = [];
  const skipped = [];

  for (const artifact of ARTIFACTS) {
    if (!profiles.includes(artifact.profile)) {
      continue;
    }
    if (!artifact.platforms.includes(PLATFORM)) {
      skipped.push({
        id: artifact.id,
        label: artifact.label,
        reason: `当前平台（${PLATFORM}）不适用`,
      });
      continue;
    }
    const rawLimit = budget[artifact.id];
    if (rawLimit === undefined) {
      skipped.push({ id: artifact.id, label: artifact.label, reason: "size-budget.json 未配置上限" });
      continue;
    }
    const limit = parseLimitForPlatform(rawLimit, artifact.id, PLATFORM);
    if (limit === null) {
      skipped.push({ id: artifact.id, label: artifact.label, reason: `当前平台（${PLATFORM}）未配置上限` });
      continue;
    }
    const paths = resolveArtifactPaths(options.root, artifact);
    if (paths.length === 0) {
      const globDirExists =
        artifact.globDir !== undefined &&
        existsSync(resolve(options.root, ...artifact.globDir.split("/")));
      skipped.push({
        id: artifact.id,
        label: artifact.label,
        reason: globDirExists ? `目录存在但没有 ${artifact.suffix} 产物` : "未构建",
      });
      continue;
    }
    for (const absolutePath of paths) {
      const size = pathSize(absolutePath);
      results.push({
        id: artifact.id,
        label: artifact.label,
        status: size > limit ? "over" : "ok",
        path: displayPath(options.root, absolutePath),
        size,
        limit,
        usage: limit > 0 ? Number((size / limit).toFixed(4)) : null,
      });
    }
  }

  const over = results.filter((entry) => entry.status === "over");
  return {
    ok: over.length === 0,
    platform: PLATFORM,
    profiles,
    results,
    skipped,
    summary: {
      checked: results.length,
      ok: results.length - over.length,
      over: over.length,
      skipped: skipped.length,
    },
  };
}

/** 人类可读输出。 */
function printReport(report, options) {
  const out = [];
  out.push("");
  out.push(`Rust 构建产物体积检查（平台：${report.platform}，档位：${report.profiles.join(" + ")}）`);
  out.push("─".repeat(88));

  if (report.results.length === 0) {
    out.push("未发现可检查的构建产物；先执行 `cargo build` / `npm run tauri build` 再检查。");
  } else {
    const labelWidth = Math.max(...report.results.map((entry) => displayWidth(entry.label)));
    for (const entry of report.results) {
      const mark = entry.status === "ok" ? "[通过]" : "[超限]";
      const usage = `${Math.round(entry.usage * 100)}%`;
      out.push(
        `${mark} ${padDisplay(entry.label, labelWidth)}  ` +
          `${formatBytes(entry.size).padStart(10)} / ${formatBytes(entry.limit).padStart(10)}  ` +
          `${usage.padStart(4)}  ${entry.path}`,
      );
    }
  }

  if (report.skipped.length > 0) {
    for (const entry of report.skipped) {
      out.push(`[跳过] ${entry.label}：${entry.reason}`);
    }
  }
  out.push("─".repeat(88));

  if (report.ok) {
    const parts = [`${report.summary.checked} 项产物在上限内`];
    if (report.summary.skipped > 0) {
      parts.push(`${report.summary.skipped} 项跳过`);
    }
    out.push(`通过：${parts.join("，")}。`);
  } else {
    out.push(`失败：${report.summary.over} 项产物超出体积上限（见上表）。`);
    out.push(`调整上限请编辑 ${displayPath(options.root, options.budget)}；清理产物可执行 npm run clean:rust。`);
  }
  process.stdout.write(`${out.join("\n")}\n`);
}

function main() {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
  } catch (error) {
    if (error instanceof UsageError) {
      process.stderr.write(`${error.message}\n\n${HELP}`);
      return 2;
    }
    throw error;
  }
  if (options.help) {
    process.stdout.write(HELP);
    return 0;
  }
  let report;
  try {
    report = runChecks(options);
  } catch (error) {
    if (error instanceof UsageError) {
      process.stderr.write(`${error.message}\n`);
      return 2;
    }
    throw error;
  }
  if (options.json) {
    process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  } else {
    printReport(report, options);
  }
  return report.ok ? 0 : 1;
}

process.exitCode = main();
