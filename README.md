# CodexManager

**让网页版 ChatGPT 成为服务器科研项目的智能入口。** CodexManager 帮助已获授权的 GPT 用户在熟悉的对话体验中安全连接、理解并推进远端科研工作；Codex 额度管理、服务器账号一键切换和会话恢复，则为持续研究提供辅助保障。

English: **Bring authorized ChatGPT web workflows to server-side research projects.** CodexManager provides a secure local-first bridge for advancing remote research, with Codex usage management, account switching, and session recovery as supporting capabilities.

## 能做什么

- **ChatGPT 科研接入**：将已获授权的 ChatGPT 网页工作流接入私有网络边界内的服务器科研项目，让对话、工作区与工具策略围绕同一研究任务协作。
- **账号与额度**：通过官方设备授权添加账号，显示实际生效身份、套餐信号与额度快照，并安全切换服务器上的 Codex 凭据。
- **会话与恢复**：只读取 `state_5.sqlite` 元数据来搜索本地会话，生成 `codex resume` 命令；必要时对单个 provider 索引做可审计、可备份的修复。
- **LabContext 工作区**：登记科研工作区、查看模型可见资产、管理工具策略、验证模型可见结果，并跟踪分析任务和研究图。
- **私有网关**：自托管 Web 壳可代理受保护的 OpenAI 兼容 `/v1` 请求。它只适合你已获授权的私有部署，不是公共 API 托管服务。

## 重要边界

- 默认部署仅监听服务器回环地址；管理 RPC、数据库、账号令牌和 LabContext 管理令牌不应公开。
- 项目当前**不提供 MCP `/mcp` endpoint**。`plugins/codexmanager-connector` 是安全部署与连接指引模板，不会把 `/v1` 或管理 RPC 冒充 MCP。
- Secure MCP Tunnel 仅适合私有开发连接；公开 MCP/插件需要独立、经过审计的 HTTPS MCP 服务、逐用户授权和滥用防护。详见 [Tunnel 与插件说明](docs/open-source/03-openai-plugin-and-tunnel.md)。
- 请只使用你有权使用的 OpenAI、Codex 和 LabContext 账号与服务，并遵守其适用条款。

## 快速开始：安全自托管

要求：Docker Compose v2。

```bash
cp deploy/.env.example deploy/.env
umask 077
openssl rand -base64 32 > deploy/.web-access-password
chmod 600 deploy/.web-access-password
```

在 `deploy/.env` 中把 `CODEXMANAGER_WEB_ACCESS_PASSWORD_FILE_HOST` 设为上一步生成文件的绝对路径，然后启动：

```bash
docker compose --env-file deploy/.env -f deploy/docker-compose.self-hosted.yml up -d --build
curl -fsS http://127.0.0.1:48761/__auth_status
```

服务只发布 `127.0.0.1:48761`。从自己的电脑访问时使用 SSH 转发：

```bash
ssh -N -L 48761:127.0.0.1:48761 your-user@your-server
```

然后打开 `http://127.0.0.1:48761/` 并使用该密码登录。密码只在新数据卷首次启动时写入为哈希；要有意轮换它，请替换密码文件、将 `CODEXMANAGER_WEB_ACCESS_PASSWORD_RESET=1` 运行一次并随后恢复为 `0`。在任何反向代理或公网入口之前都应先验证登录。

根目录的 `docker-compose.yml` 采用同样的安全默认值；复用上面的 `deploy/.env` 后可运行 `docker compose --env-file deploy/.env up -d --build`。

## LabContext 连接

原生服务默认连接 `http://127.0.0.1:1455/admin`，可通过以下环境变量覆盖：

```bash
export LABCONTEXT_ADMIN_URL=http://127.0.0.1:1455/admin
export LABCTX_ADMIN_TOKEN_FILE="$HOME/.local/state/labcontext/admin.token"
```

Docker 部署需要把管理员 token 以只读方式挂入容器。请按 [服务器部署说明](docs/open-source/01-server-deployment.md) 使用 `deploy/docker-compose.labcontext.example.yml` 覆盖文件；它仅允许 `host.docker.internal` 这一 Docker 宿主机网关，不接受任意远程管理地址。

## 本地开发

要求：Node.js 20+、Python 3、Rust stable；桌面打包还需要对应平台的 Tauri 依赖。

```bash
corepack pnpm@10.30.3 -C apps install --frozen-lockfile
corepack pnpm@10.30.3 -C apps run build:desktop
corepack pnpm@10.30.3 -C apps run test:runtime
cargo test --workspace --locked
cargo test --manifest-path apps/src-tauri/Cargo.toml --locked --lib
```

更多验证和发布命令见 [TESTING.md](TESTING.md) 与 [公开发布清单](docs/open-source/04-public-release-checklist.md)。

## 桌面端自动更新

桌面端默认不查询任何 GitHub 仓库。创建自己的 GitHub Release 后，才在桌面端运行环境（或其环境变量覆盖）中设置：

```bash
CODEXMANAGER_UPDATE_REPO=owner/repository
```

留空会关闭更新检查和下载准备，避免安装包在未经维护者确认的情况下跟随上游项目。需要接收预发布版本时，再显式设置 `CODEXMANAGER_UPDATE_PRERELEASE=1` 并重启应用。

## 目录

```text
apps/                    Next.js 前端与 Tauri 桌面壳
crates/core/             SQLite 迁移、存储与认证基础
crates/service/          本地服务、账户、会话、网关与 LabContext 适配
crates/web/              Web 运行壳和受保护的 RPC 代理
crates/start/            all-in-one 启动器
deploy/                  安全 Compose 与可选 LabContext 覆盖
plugins/                 不含连接 ID 的 Codex 插件模板
docs/open-source/        部署、Tunnel 与发布说明
cm-skills/               可选的 CodexManager Images API 本地技能
```

## 安全与贡献

- 安全问题请遵循 [SECURITY.md](SECURITY.md)，不要在公开 Issue 中提交凭据或可利用细节。
- 贡献流程和验证要求见 [CONTRIBUTING.md](CONTRIBUTING.md)。
- 发布前运行 `scripts/open-source/preflight.sh`，并对最终 Git 历史执行独立秘密扫描。

## 许可与来源

本项目采用 [MIT License](LICENSE)。它基于 `qxcnm/Codex-Manager` 的 MIT 许可代码演进而来；版权与来源说明见 [NOTICE](NOTICE)。
