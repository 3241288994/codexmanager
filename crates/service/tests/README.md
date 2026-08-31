# crates/service tests 说明

## 目标

本目录承载 service crate 的 Rust 测试，按“单元 / 集成 / 协议兼容”三层理解最清晰。

## 当前分层

### 根目录测试文件

- `app_settings.rs`
- `default_addr.rs`
- `gateway_logs.rs`
- `rpc.rs`
- `shutdown_flag.rs`
- `e2e.rs`

职责：

- service 对外门面测试
- 配置默认值与运行时行为测试
- 跨模块集成验证
- 最小 e2e 路径验证

### `auth/`

职责：

- OAuth / 回调相关测试
- 登录链路局部回归

### `usage/`

职责：

- 用量刷新状态与相关回归
- free 账号无 `refresh_token` 时的刷新保护回归

### `gateway/`

职责：

- 网关选路
- 可用性判定
- 协议兼容
- 上游头部与故障切换

其中 `gateway/availability/` 当前已是高价值兼容回归子域。

## 推荐理解方式

### 单元测试

适合：

- 纯函数
- 无需真实 HTTP / RPC 的局部逻辑
- 配置归一化与状态机

### HTTP / RPC 集成测试

适合：

- 对外接口与内部模块的组合行为
- `app_settings_get/set`
- `rpc` 调度与 shutdown 行为
- 默认地址、日志记录等门面能力

### 协议兼容回归测试

适合：

- `/v1/chat/completions`
- `/v1/responses`
- stream / non-stream
- tools / tool_calls
- 协议转换与聚合行为

这类测试优先沉淀到 `gateway/` 子目录，避免散落在 crate 根。

## 运行建议

- 最小检查：`cargo test -p codexmanager-service --lib`
- service 测试：`cargo test -p codexmanager-service`
- 全工作区：`cargo test --workspace`

## 维护约定

- 新增协议兼容测试，优先放到 `tests/gateway/`
- 新增 app settings / runtime sync 测试，优先放到根目录或后续专门子目录
- 若测试需要大量 fixture，优先新建子目录，不要继续把 crate 根测试文件堆大
- 新增模型目录 / `models_cache.json` 相关回归时，优先明确放到 `usage/` 或 `gateway/` 之下，不要把桌面端同步假设散落到普通门面测试里
