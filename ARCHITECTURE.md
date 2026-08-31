# Architecture

## 产品边界

CodexManager 有三个面向用户的领域：

```text
Browser / Tauri
  ├─ Accounts & Usage
  │   ├─ verified Codex identity and usage snapshots
  │   └─ explicit credential switching
  ├─ Sessions & Recovery
  │   ├─ read-only state_5.sqlite catalog
  │   ├─ official resume command generation
  │   └─ one-row provider-index repair with backup
  └─ LabContext
      ├─ workspace registry and model-visible asset coverage
      ├─ tool policy, jobs, audit records, and research maps
      └─ loopback-only LabContext admin adapter
```

账户切换不会改写 LabContext 工作区。界面选中的工作区只影响控制台的本地导航；当模型工具请求显式携带 `workspace_id` 时，该值优先于默认工作区。

## 运行时

`codexmanager-web` 提供静态 Next.js UI，并将受认证保护的 JSON-RPC 转发至 `codexmanager-service`。桌面端通过 Tauri 命令调用同一组服务能力；Web 端通过 `/api/runtime` 和 `/api/rpc` 使用等价的命令映射。

服务拥有账号存储、OpenAI 设备授权和用量刷新、Codex profile 写入、本地会话索引、LabContext 控制面适配以及可选 OpenAI 兼容网关。`codexmanager-start` 将 service 与 web 壳组合成单一进程组。

## 关键不变量

### 账号切换

```text
selected account
  → stage auth/config/marker
  → persist Codex home
  → read auth.json actual account id
  → verify selected == actual
  → success or rollback
```

不会改写会话目录或 JSONL 正文。已有 Codex 进程仍可能保留内存中的旧凭据，因此应启动新的 Codex 进程后再验证。

### 会话恢复

会话列表只读打开 `state_5.sqlite`，对不同 Codex 版本的可选列保持兼容，不扫描 rollout 正文。provider 索引修复要求精确确认 Session ID，并在更新一行前创建 SQLite 备份和 JSON 审计记录；rollout 文件永不被修改。

### LabContext

LabContext 管理端默认只能使用 HTTP loopback 地址。容器部署时可使用 Docker 的 `host.docker.internal` 网关，并且必须将管理员 token 以只读文件挂载。任意远程主机、HTTPS URL 和模型可见管理接口都被拒绝。

## 信任边界

- Web 访问认证保护管理 RPC。
- 在 `accounts` 多用户认证模式中，成员只能访问自己的 API Key、用量摘要与请求日志；服务器账号、Codex 配置、会话目录、LabContext 管理面及全局设置均要求管理员角色。
- Service 与 Web 默认应只监听 loopback；公网部署需要独立的 TLS、认证、限流与审计设计。
- 账号 token、RPC token、平台 Key、Codex 状态和 LabContext 管理 token 都是本地秘密，不能提交或写入普通日志。
- `/v1` 是可选的私有客户端接口；它不是 MCP。ChatGPT 模型工具必须通过独立、最小权限的 MCP 服务提供。
