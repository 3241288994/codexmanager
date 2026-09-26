# 统一 LabContext Router

CodexManager 可以把相互独立的本机 LabContext 和服务器 LabContext 放在一个可选 Router 后面，
同时保持安装方式解耦：只装本机、只连服务器、两者并用都使用同一套命令和同一个 MCP 地址。

```text
ChatGPT 网页版 ─ Secure MCP Tunnel ─┐
                                    ├─ 127.0.0.1:1460/mcp (Router)
CodexManager Web 控制台 ────────────┘       ├─ local Provider  ─ 本机目录
                                             └─ server Provider ─ SSH 转发 ─ 服务器目录
```

Router 不包含 LabContext 本身，也不要求 Provider 由同一种方式安装。每个 Provider 只需提供兼容的
`/mcp`；需要在 CodexManager Web 页面管理工作区时，再为它配置 `/admin` 和私有 token 文件。

## 安装与初始化

要求 Python 3.9+。安装脚本只复制两个零依赖 Python 文件，不会安装或启动任何 Provider：

```bash
scripts/install-labcontext.sh
```

选择符合需要的模式进行一次性初始化：

```bash
# 仅本机
labcontext init local --tunnel-profile labcontext

# 仅服务器；SSH Host 可以是 ~/.ssh/config 中的别名
labcontext init server --ssh-host your-server --tunnel-profile labcontext

# 本机 + 服务器
labcontext init hybrid --ssh-host your-server --tunnel-profile labcontext
```

生成的文件是：

- `~/.config/labcontext/config.toml`：Provider、端口和允许访问 Router 的 Web Origin；
- `~/.config/labcontext/launcher.env`：可选 SSH/Tunnel 进程设置，权限自动设为 `0600`。

初始化不会覆盖已有配置；确实需要重建时才使用 `--force`。仓库中的
`deploy/labcontext-router.example.toml` 展示了完整双 Provider 配置。

## Provider 端口

| 端口 | 用途 |
| --- | --- |
| `1455` | server Provider，或单 Provider 模式下的 LabContext |
| `1456` | hybrid 模式下的 local Provider |
| `1460` | 统一 Router 的 `/mcp`、健康检查和本机 Web 管理桥 |
| `48761` | CodexManager Web UI；服务器模式下可由同一 SSH 连接转发 |

在 hybrid 模式中，本机和转发后的服务器端口不能冲突。若本机 LabContext 仍监听 `1455`，请将它
改到 `1456`，或相应修改 `config.toml` 与 `launcher.env` 中的 server 转发端口。

每个启用了管理桥的 Provider 都要有对应 token 文件。token 不应写入 TOML、Git 或浏览器；
Router 只从 `admin_token_file` 读取它，并只监听 IPv4 回环地址。

需要指定 SSH identity、绕过个人 SSH config，或保留服务器到本机代理的反向转发时，可在私有
`launcher.env` 设置 `LABCONTEXT_SSH_IDENTITY_FILE`、`LABCONTEXT_SSH_CONFIG_FILE`、
`LABCONTEXT_SERVER_PROXY_REMOTE_PORT` 与 `LABCONTEXT_LOCAL_PROXY_PORT`。已有 API-key 文件可通过
`LABCONTEXT_SECRET_ENV_FILE` 引用，不需要复制密钥内容。

## Tunnel 与日常启动

Tunnel profile 应指向 Router，而不是某个 Provider：

```bash
tunnel-client init \
  --profile labcontext \
  --tunnel-id tunnel_your_id \
  --mcp-server-url http://127.0.0.1:1460/mcp
```

把 `CONTROL_PLANE_API_KEY` 注入当前环境，或写入权限为 `0600` 的私有
`launcher.env`。先检查配置，再启动全部已启用组件：

```bash
labcontext doctor
labcontext
```

`labcontext` 会按配置启动 SSH 转发、Router 和 Tunnel，并在退出时回收这些子进程。Provider
默认仍由各自的 launchd、systemd、Docker 或手工命令管理；若希望同一命令顺便启动独立安装的
local Provider，可在 `launcher.env` 设置不经过 shell 展开的
`LABCONTEXT_LOCAL_PROVIDER_COMMAND`。原有个人启动脚本可以保留，确认新 profile 可用后再停用。

## ChatGPT 中的调用方式

连接这一个 Tunnel 后，先调用 `list_workspaces`。Router 会给每个结果增加稳定的
`workspace_ref`，例如 `local:paper-a` 或 `server:paper-a`。之后将这个值传给
`workspace_overview`、`research_context` 等工具；异步任务返回的 `job_id` 也会带 Provider 前缀。

当只有一个 Provider 暴露某工具时，Router 可以自动选择它；当两个 Provider 中存在同名
`workspace_id` 时，必须使用 `workspace_ref`，以免查询到错误电脑上的项目。某个 Provider 临时
离线不会阻止另一个 Provider 的工作区被列出，返回结果会同时标记各来源状态。

## Web 控制台

浏览器打开 `http://127.0.0.1:48761/labcontext/` 时会探测
`http://127.0.0.1:1460/api/providers`。如果 `local` Provider 的 MCP 和 admin 都可用，页面会自动
解锁“本地电脑”；不需要切换到 Tauri 桌面版。桌面版仍保留原生目录选择器，Web 版则要求填写
运行 local Provider 的电脑能够访问的绝对路径。

如果 CodexManager Web 使用了其他 Origin，必须把精确 Origin 加入 `cors_origins` 后重启
Router。不要使用通配 Origin，也不要把 `1460`、Provider 的 `/admin` 或 token 暴露到局域网或
公网。

## 故障定位

```bash
labcontext doctor
curl -fsS http://127.0.0.1:1460/healthz
```

健康响应会分别显示 `local` / `server` 的 MCP `status` 与 `adminStatus`。常见问题是 Provider
未启动、SSH 转发端口冲突、admin token 文件路径错误，或 Tunnel profile 仍指向旧的
`1455/mcp`。
