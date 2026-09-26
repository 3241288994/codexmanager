# 本地科研工作区

CodexManager 桌面版可以在同一个“科研工作区”页面中切换两套独立连接：

```text
服务器工作区：桌面/Web UI → CodexManager service → 服务器 LabContext
本地工作区：  桌面 UI     → Tauri 回环适配 → 本机 LabContext
```

本地模式适合先在个人电脑上整理代码、论文和实验记录，再让已获授权的 ChatGPT 工作流通过
本机 MCP 连接查询它们。服务器与本机的默认工作区、工具策略、任务、研究图和审计记录互不混合。

## 前提

- 使用 CodexManager **桌面版**。普通网页没有读取浏览器所在电脑目录的权限，因此 Web/Docker
  版中的“本地电脑”入口会保持禁用。
- 在当前电脑上另行安装并启动兼容的 LabContext，且其管理员控制面监听 HTTP 回环地址。
- 准备 LabContext admin token。默认读取：
  `~/.local/state/labcontext/admin.token`。

默认地址为 `http://127.0.0.1:1455/admin`。需要覆盖时，在启动桌面应用前设置：

```bash
export CODEXMANAGER_LOCAL_LABCONTEXT_ADMIN_URL=http://127.0.0.1:1455/admin
export CODEXMANAGER_LOCAL_LABCONTEXT_ADMIN_TOKEN_FILE=/absolute/private/path/admin.token
```

自定义地址仍必须使用 `http://127.0.0.1`、`http://localhost` 或 `http://[::1]`；桌面适配不会
连接局域网、Docker host gateway 或公网管理端。token 文件应仅允许当前用户读取，也不要把它
放入项目目录。

## 添加本机项目

1. 打开桌面版的“科研工作区”，切换到“本地电脑”。
2. 点击“添加工作区”，选择“本地电脑”，填写名称并点击“选择文件夹”。
3. 在系统文件夹选择器中选择具体项目目录，再点击“添加并自动配置”。
4. 检查工作区概述、模型可见资产和工具策略，并运行“测试 ChatGPT 所见内容”。

本地路径不能手工输入。桌面壳会规范化系统选择器返回的路径，只允许本次明确选中的目录注册，
并拒绝文件系统根目录和整个用户主目录。删除注册不会删除项目文件或
`.labcontext/context.yaml`。

## 让 ChatGPT 网页访问本地项目

在 CodexManager 中添加本地目录只完成了**管理连接**；它不会自动把本机目录发布给 ChatGPT。
ChatGPT 网页也不能直接访问 `localhost`。应在本机完成一条独立的模型工具链：

```text
ChatGPT 网页版
  → 本机专用 Secure MCP Tunnel
  → 本机上经过审计的 MCP 适配层
  → 本机 LabContext 的受限模型工具
  → 已显式注册的项目目录
```

在 OpenAI Platform 为本机连接创建或选择 Tunnel，然后在**当前电脑**上运行
`tunnel-client`；它所指向的必须是本机 MCP 服务的 `/mcp` 地址，而不是 CodexManager 的
`/api/rpc`、LabContext admin 地址或 SSH 转发。例如：

```bash
export CONTROL_PLANE_API_KEY="$(your-secret-manager-read-command)"
tunnel-client init \
  --profile codexmanager-local \
  --tunnel-id tunnel_your_local_id \
  --mcp-server-url http://127.0.0.1:3000/mcp
tunnel-client doctor --profile codexmanager-local --explain
tunnel-client run --profile codexmanager-local
```

随后在 ChatGPT developer mode 的 Plugins 中选择这条 Tunnel，并确认发现的工具只包含需要的
读取能力和经过确认的写入能力。现有的服务器 Tunnel 不会自动转发到个人电脑；若要同时使用
服务器和本机项目，应分别维护两条连接并给出清晰名称。完整步骤与公开插件边界见
[OpenAI 插件与 Secure MCP Tunnel](open-source/03-openai-plugin-and-tunnel.md)。

## 安全边界

- 本地管理命令只在 Tauri 桌面壳注册，不映射到 Web RPC。
- 管理连接禁用系统代理，只允许回环 HTTP 地址，admin token 不返回前端。
- 添加目录必须经过原生选择器的一次性授权；任意路径不能通过通用调用注册。
- CodexManager 当前仍不提供 MCP `/mcp` endpoint。模型连接必须使用独立、最小权限、可审计的
  MCP 适配层。
- 工作区授权不等于允许模型读取项目中的所有内容；最终可见范围仍由 LabContext 资产规则和
  工具策略决定。
