# 本地电脑接入

## 连接私有 CodexManager 网关

先完成以下前提：

1. 服务器上的 CodexManager 已启动，并已启用 Web 访问认证。
2. 你已建立 SSH 转发，并能在本机打开 `http://127.0.0.1:48761/`。
3. 你在 CodexManager 的平台密钥管理中创建了专用 Key。

在本机 `~/.codex/auth.json`（Windows 为 `%USERPROFILE%\\.codex\\auth.json`）中使用该
平台 Key：

```json
{
  "OPENAI_API_KEY": "replace-with-your-codexmanager-platform-key",
  "auth_mode": "apikey"
}
```

然后在同目录的 `config.toml` 配置私有提供商。模型名必须是你的部署实际允许的模型：

```toml
model = "replace-with-an-enabled-model"
model_provider = "codexmanager"

[model_providers.codexmanager]
name = "CodexManager"
base_url = "http://127.0.0.1:48761/v1"
wire_api = "responses"
```

重启 Codex CLI 后再验证。不要把 OpenAI 登录的 access token、refresh token、服务器
`codexmanager.rpc-token` 或 LabContext admin token 当作平台 Key。

这些文件只属于本机用户配置，绝不能复制到本仓库、插件目录、截图或 issue 中。

## 本地开发

要求：Node.js 20+、Corepack、Python 3 和 Rust stable。桌面打包还需要对应平台的 Tauri
依赖。

```bash
corepack pnpm@10.30.3 -C apps install --frozen-lockfile
corepack pnpm@10.30.3 -C apps run build
corepack pnpm@10.30.3 -C apps run test:runtime
corepack pnpm@10.30.3 -C apps run test:e2e
cargo test --workspace
```

本仓库不携带 `node_modules`、Rust `target`、Next.js 输出、数据库或运行时凭据；它们应在
自己的开发环境中重新生成。提交或发布前运行：

```bash
scripts/open-source/preflight.sh
```

该脚本会拒绝常见的秘密、数据库、个人插件连接和已知个人部署引用；它不能替代对最终
Git 历史运行的专用秘密扫描器。

## 桌面端更新源

公开源码不会预置任何更新仓库。你创建自己的 GitHub Release 后，才为桌面端进程设置
`CODEXMANAGER_UPDATE_REPO=owner/repository`，然后重启应用。留空时更新检查与下载准备会明确
提示尚未配置，而不会访问上游项目。预发布更新也必须显式设置
`CODEXMANAGER_UPDATE_PRERELEASE=1`。
