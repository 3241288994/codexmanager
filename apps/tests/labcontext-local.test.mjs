import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const appsRoot = path.resolve(import.meta.dirname, "..");

async function readSource(relativePath) {
  return fs.readFile(path.join(appsRoot, relativePath), "utf8");
}

test("本地科研工作区只通过桌面壳暴露", async () => {
  const [client, page, hook, webCommands, registry] = await Promise.all([
    readSource("src/lib/api/labcontext-client.ts"),
    readSource("src/app/labcontext/page.tsx"),
    readSource("src/hooks/useLabContextWorkspace.ts"),
    readSource("src/lib/api/transport-web-commands.ts"),
    readSource("src-tauri/src/commands/registry.rs"),
  ]);

  for (const command of [
    "app_labcontext_pick_local_workspace_directory",
    "app_labcontext_local_call",
    "app_labcontext_local_upsert_workspace",
  ]) {
    assert.match(client, new RegExp(command));
    assert.match(registry, new RegExp(`::${command}\\b`));
    assert.doesNotMatch(webCommands, new RegExp(command));
  }

  assert.match(page, /useLabContextWorkspace\(isDesktopRuntime\)/);
  assert.match(hook, /labContextClient\.pickLocalWorkspaceDirectory/);
  assert.match(page, /localEnabled=\{isDesktopRuntime\}/);
  assert.match(page, /Web (?:版|模式)不能读取(?:浏览器所在)?电脑的目录/);
});

test("科研工作区在隐藏页面暂停轮询并优化目录选择", async () => {
  const hook = await readSource("src/hooks/useLabContextWorkspace.ts");

  assert.match(hook, /document\.visibilityState === "visible"/);
  assert.match(hook, /refetchIntervalInBackground:\s*false/);
  assert.match(hook, /staleTime:\s*5_000/);
  assert.match(hook, /suggestedWorkspaceName\(result\.path/);
  assert.match(hook, /window\.addEventListener\("scroll", close, true\)/);
});

test("本地目录注册经过原生选择器和回环地址限制", async () => {
  const localCommands = await readSource("src-tauri/src/commands/labcontext_local.rs");
  const productionSource = localCommands.split("#[cfg(test)]", 1)[0];

  assert.match(localCommands, /FileDialog::new\(\)[\s\S]*?\.pick_folder\(\)/);
  assert.match(localCommands, /fs::canonicalize/);
  assert.match(localCommands, /ensure_selected_root/);
  assert.match(localCommands, /不能把文件系统根目录授权为科研工作区/);
  assert.match(localCommands, /不能把整个用户目录授权为科研工作区/);
  assert.match(productionSource, /127\.0\.0\.1/);
  assert.match(productionSource, /localhost/);
  assert.match(productionSource, /"::1"/);
  assert.doesNotMatch(productionSource, /host\.docker\.internal/);
});

test("通用本地调用不允许绕过目录授权创建工作区", async () => {
  const localCommands = await readSource("src-tauri/src/commands/labcontext_local.rs");
  const genericCall = localCommands.match(/fn call_local_operation[\s\S]*?\n\}/)?.[0];

  assert.ok(genericCall, "未找到本地 LabContext 通用调用函数");
  assert.doesNotMatch(genericCall, /"upsertWorkspace"\s*=>/);
  assert.match(localCommands, /pub async fn app_labcontext_local_upsert_workspace/);
});
