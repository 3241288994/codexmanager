import { existsSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import net from "node:net";

const cwd = process.cwd();
const task = process.argv[2] || "build:desktop";
const desktopDevHost = "127.0.0.1";
const desktopDevPort = 3005;
const desktopDevWaitTimeoutMs = 10_000;
const desktopDevWaitIntervalMs = 500;
const candidates = [
  cwd,
  resolve(cwd, "apps"),
  resolve(cwd, "..", "apps"),
  resolve(cwd, "..", "..", "apps"),
  resolve(cwd, ".."),
  resolve(cwd, "..", ".."),
];

function hasFrontendPackage(dir) {
  return existsSync(resolve(dir, "package.json"));
}

function hasBuiltFrontendDist(dir) {
  return existsSync(resolve(dir, "out", "index.html"));
}

function canConnect(host, port, timeoutMs = 1000) {
  return new Promise((resolvePromise) => {
    const socket = new net.Socket();
    let settled = false;

    const finish = (result) => {
      if (settled) {
        return;
      }
      settled = true;
      socket.destroy();
      resolvePromise(result);
    };

    socket.setTimeout(timeoutMs);
    socket.once("connect", () => finish(true));
    socket.once("timeout", () => finish(false));
    socket.once("error", () => finish(false));
    socket.connect(port, host);
  });
}

async function hasReusableDesktopDevServer() {
  const reachable = await canConnect(desktopDevHost, desktopDevPort);
  if (!reachable) {
    return false;
  }

  try {
    const response = await fetch(`http://${desktopDevHost}:${desktopDevPort}`, {
      signal: AbortSignal.timeout(1500),
    });
    return response.ok;
  } catch {
    return false;
  }
}

function getDesktopDevLockPath(dir) {
  return resolve(dir, ".next", "dev", "lock");
}

function sleep(ms) {
  return new Promise((resolvePromise) => {
    setTimeout(resolvePromise, ms);
  });
}

async function waitForDesktopDevPortToClose(timeoutMs = 5_000) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() <= deadline) {
    if (!(await canConnect(desktopDevHost, desktopDevPort, 300))) {
      return true;
    }

    if (Date.now() >= deadline) {
      return false;
    }

    await sleep(250);
  }

  return false;
}

async function waitForReusableDesktopDevServer(timeoutMs = desktopDevWaitTimeoutMs) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() <= deadline) {
    if (await hasReusableDesktopDevServer()) {
      return true;
    }

    if (Date.now() >= deadline) {
      return false;
    }

    await sleep(desktopDevWaitIntervalMs);
  }

  return false;
}

function listDesktopDevListenerPids() {
  const result = spawnSync("netstat", ["-ano", "-p", "tcp"], {
    encoding: "utf8",
  });

  if (result.error || result.status !== 0) {
    return [];
  }

  const expectedAddress = `${desktopDevHost}:${desktopDevPort}`;
  const pids = new Set();

  for (const line of result.stdout.split(/\r?\n/)) {
    if (!line.includes(expectedAddress) || !/\bLISTENING\b/i.test(line)) {
      continue;
    }

    const match = line.trim().match(/(\d+)$/);
    if (!match) {
      continue;
    }

    pids.add(Number.parseInt(match[1], 10));
  }

  return [...pids];
}

function getWindowsProcessInfo(pid) {
  const command = [
    `$process = Get-CimInstance Win32_Process -Filter "ProcessId = ${pid}" -ErrorAction SilentlyContinue`,
    'if ($process) { $process | Select-Object ProcessId, ParentProcessId, CommandLine | ConvertTo-Json -Compress }',
  ].join("; ");

  const result = spawnSync("powershell.exe", ["-NoProfile", "-Command", command], {
    encoding: "utf8",
  });

  if (result.error || result.status !== 0) {
    return null;
  }

  const rawOutput = result.stdout.trim();
  if (!rawOutput) {
    return null;
  }

  try {
    return JSON.parse(rawOutput);
  } catch {
    return null;
  }
}

function isDesktopDevProcess(pid) {
  let currentPid = pid;

  for (let index = 0; index < 4 && currentPid; index += 1) {
    const processInfo = getWindowsProcessInfo(currentPid);
    if (!processInfo?.CommandLine) {
      break;
    }

    const normalizedCommandLine = processInfo.CommandLine.toLowerCase();
    const isNextProcess =
      normalizedCommandLine.includes("next dev") ||
      normalizedCommandLine.includes("\\next\\dist\\bin\\next") ||
      normalizedCommandLine.includes("start-server.js");
    const matchesDesktopPort =
      normalizedCommandLine.includes(`-p ${desktopDevPort}`) ||
      normalizedCommandLine.includes(`:${desktopDevPort}`);

    if (isNextProcess && (matchesDesktopPort || index > 0)) {
      return true;
    }

    currentPid = processInfo.ParentProcessId;
  }

  return false;
}

function terminateWindowsProcessTree(pid) {
  const result = spawnSync("taskkill", ["/PID", String(pid), "/T", "/F"], {
    encoding: "utf8",
  });

  if (result.error) {
    return false;
  }

  if (result.status === 0) {
    return true;
  }

  const combinedOutput = `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
  return /not found|no running instance|does not exist/i.test(combinedOutput);
}

async function cleanupStaleDesktopDevState() {
  const listenerPids = listDesktopDevListenerPids();

  for (const pid of listenerPids) {
    const processInfo = getWindowsProcessInfo(pid);
    if (!isDesktopDevProcess(pid)) {
      console.error(
        `端口 ${desktopDevPort} 被其他进程占用，无法自动清理。PID: ${pid}，命令行: ${processInfo?.CommandLine || "未知"}`,
      );
      process.exit(1);
    }

    console.log(`检测到未响应的 Next.js 开发进程，准备终止: PID ${pid}`);
    if (!terminateWindowsProcessTree(pid)) {
      console.error(`终止残留 Next.js 开发进程失败: PID ${pid}`);
      process.exit(1);
    }
  }

  if (listenerPids.length > 0) {
    const portReleased = await waitForDesktopDevPortToClose();
    if (!portReleased) {
      console.error(`端口 ${desktopDevPort} 释放超时，无法继续启动前端开发服务`);
      process.exit(1);
    }
  }

  const desktopDevLockPath = getDesktopDevLockPath(frontendDir);
  if (existsSync(desktopDevLockPath)) {
    try {
      rmSync(desktopDevLockPath, { force: true });
      console.log(`已清理未响应的 Next.js 开发锁文件: ${desktopDevLockPath}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`清理 Next.js 开发锁文件失败: ${message}`);
      process.exit(1);
    }
  }
}

function resolvePnpmCommand() {
  const baseArgs = ["--dir", frontendDir, "run", task];
  const nodeBinDir = dirname(process.execPath);
  const windowsCandidates = [
    { command: resolve(nodeBinDir, "pnpm.cmd"), args: baseArgs },
    { command: resolve(nodeBinDir, "corepack.cmd"), args: ["pnpm", ...baseArgs] },
    { command: "pnpm.cmd", args: baseArgs },
    { command: "corepack.cmd", args: ["pnpm", ...baseArgs] },
  ];
  const defaultCandidates = [
    { command: "pnpm", args: baseArgs },
    { command: "corepack", args: ["pnpm", ...baseArgs] },
  ];

  const candidates = process.platform === "win32" ? windowsCandidates : defaultCandidates;
  const existingPathCandidates = candidates.filter(
    (candidate) => !candidate.command.includes(":") || existsSync(candidate.command),
  );

  for (const candidate of existingPathCandidates) {
    const probeArgs = candidate.args[0] === "pnpm" ? ["pnpm", "--version"] : ["--version"];
    const probe = spawnSync(candidate.command, probeArgs, {
      encoding: "utf8",
      shell: process.platform === "win32" && /\.cmd$/i.test(candidate.command),
      stdio: "ignore",
    });
    if (!probe.error && probe.status === 0) {
      return candidate;
    }
  }

  return candidates[candidates.length - 1];
}

const frontendDir = candidates.find(hasFrontendPackage);
if (!frontendDir) {
  console.error(`前端项目目录不存在，当前工作目录: ${cwd}`);
  process.exit(1);
}

if (task === "build:desktop" && hasBuiltFrontendDist(frontendDir)) {
  console.log(`前端产物已存在，跳过重复构建: ${resolve(frontendDir, "out", "index.html")}`);
  process.exit(0);
}

if (task === "dev:desktop") {
  if (await hasReusableDesktopDevServer()) {
    console.log(`检测到现有前端开发服务，直接复用: http://${desktopDevHost}:${desktopDevPort}`);
    process.exit(0);
  }

  const desktopDevLockPath = getDesktopDevLockPath(frontendDir);
  const hasDesktopDevLock = existsSync(desktopDevLockPath);
  const hasDesktopDevPortListener = await canConnect(desktopDevHost, desktopDevPort, 300);
  if (hasDesktopDevLock || hasDesktopDevPortListener) {
    const staleState = [hasDesktopDevLock ? "锁文件" : null, hasDesktopDevPortListener ? "端口占用" : null]
      .filter(Boolean)
      .join(" / ");
    console.log(`检测到 Next.js 开发态残留（${staleState}），等待现有实例就绪: ${desktopDevLockPath}`);

    if (await waitForReusableDesktopDevServer()) {
      console.log(`检测到现有前端开发服务，直接复用: http://${desktopDevHost}:${desktopDevPort}`);
      process.exit(0);
    }

    await cleanupStaleDesktopDevState();
  }
}

const packageManager = resolvePnpmCommand();
console.log(`执行前端任务: ${packageManager.command} ${packageManager.args.join(" ")}`);
const needsShell = process.platform === "win32" && /\.cmd$/i.test(packageManager.command);
const result = spawnSync(packageManager.command, packageManager.args, {
  stdio: "inherit",
  shell: needsShell,
});

if (result.error) {
  console.error(`前端构建启动失败: ${result.error.message}`);
  process.exit(1);
}

if (typeof result.status === "number" && result.status !== 0) {
  process.exit(result.status);
}

process.exit(0);
