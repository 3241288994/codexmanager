# 本地工作区

CodexManager 可以在同一个“工作区”页面中切换两套独立连接：

```text
服务器工作区：桌面/Web UI → CodexManager service → 服务器 LabContext
本地工作区：  桌面 UI     → Tauri 回环适配        → 本机 LabContext
               Web UI      → 可选回环 Router 管理桥 → 本机 LabContext
```

本地模式适合先在个人电脑上整理代码、论文和实验记录，再让已获授权的 ChatGPT 工作流通过
本机 MCP 连接查询它们。服务器与本机的默认工作区、工具策略、任务、研究图和审计记录互不混合。

## 前提

- 使用 CodexManager 桌面版，或在浏览器所在电脑启动可选的 LabContext Router。普通网页不能
  自己读取文件系统，但可以把明确填写的路径交给回环 Router；Router 不可用时“本地电脑”保持禁用。
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

## 路径直读：无需添加工作区

兼容 LabContext Provider 提供 `inspect_path` 后，可以直接在 ChatGPT 中粘贴绝对路径：

```text
请读取本地电脑的 /Users/me/Desktop/project/README.md。
请查看服务器 /srv/research/project 下两层目录，并总结项目入口。
```

这类请求不会修改 Provider 配置，也不会在项目中创建 `.labcontext` 文件。路径必须位于对应
Provider 的 `registry.allowed_roots` 下；hybrid 模式应明确说“本地电脑”或“服务器”。目录只返回
有限深度和有限条目，后续文件由模型按需读取，以控制内存和上下文占用。

## 固定为工作区

1. 打开“工作区”，切换到“本地电脑”。
2. 点击“添加工作区”并选择“本地电脑”。桌面版使用系统选择器；Web 版填写 local Provider
   所在电脑可访问的绝对路径。
3. 确认名称与具体项目目录，再点击“添加并自动配置”。
4. 检查工作区概述、模型可见资产和工具策略，并运行“测试 ChatGPT 所见内容”。

桌面版路径不能手工输入：桌面壳会规范化系统选择器返回的路径，并拒绝文件系统根目录和整个
用户主目录。Web + Router 模式依赖 local Provider 自身的路径校验和资产规则，因此只应在本人
电脑的回环边界内使用。删除注册不会删除项目文件或 `.labcontext/context.yaml`。

## 让 ChatGPT 网页访问本地项目

在 CodexManager 中添加本地目录只完成了**管理连接**；ChatGPT 仍需通过 Secure MCP Tunnel
访问一个 MCP endpoint。推荐让 Tunnel 指向统一 Router：

```text
ChatGPT 网页版
  → 本机专用 Secure MCP Tunnel
  → 统一 LabContext Router (`127.0.0.1:1460/mcp`)
  → 本机和/或服务器 LabContext 的受限模型工具
  → 已授权的临时绝对路径，或已固定的工作区
```

在 OpenAI Platform 为本机连接创建或选择 Tunnel，然后在**当前电脑**上运行
`tunnel-client`；它所指向的必须是本机 MCP 服务的 `/mcp` 地址，而不是 CodexManager 的
`/api/rpc`、LabContext admin 地址或 SSH 转发。例如：

```bash
export CONTROL_PLANE_API_KEY="$(your-secret-manager-read-command)"
tunnel-client init \
  --profile codexmanager-local \
  --tunnel-id tunnel_your_local_id \
  --mcp-server-url http://127.0.0.1:1460/mcp
tunnel-client doctor --profile codexmanager-local --explain
tunnel-client run --profile codexmanager-local
```

随后在 ChatGPT developer mode 的 Plugins 中选择这条 Tunnel，并确认发现的工具只包含需要的
读取能力和经过确认的写入能力。Router 可在一个连接中聚合两类 Provider；具体配置见
[统一 LabContext Router](labcontext-router.md)，完整插件步骤与公开边界见
[OpenAI 插件与 Secure MCP Tunnel](open-source/03-openai-plugin-and-tunnel.md)。

## 安全边界

- 本地管理命令不映射到服务器 Web RPC；可选 Router 只在浏览器所在电脑的 IPv4 回环地址提供
  独立、操作白名单化的管理桥。
- 管理连接禁用系统代理，只允许回环 HTTP 地址，admin token 不返回前端。
- 桌面版添加目录必须经过原生选择器的一次性授权；Web + Router 模式应依赖 Provider 的路径规则。
- CodexManager service 本身不提供 MCP endpoint；Router 是可选、独立部署的最小路由层。
- 路径直读只访问 `registry.allowed_roots` 下经过校验的文件和有限目录树；工作区工具的最终可见
  范围仍由 LabContext 资产规则和工具策略决定。
