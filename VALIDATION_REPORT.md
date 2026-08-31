# CodexManager 开源验证记录

日期：2026-08-31（Asia/Shanghai）

| 检查 | 结果 | 说明 |
| --- | --- | --- |
| 前端 lint | 通过 | `corepack pnpm@10.30.3 -C apps run lint` |
| 前端静态构建 | 通过 | `build:desktop` 成功生成三个维护路由 |
| 前端运行时测试 | 29/29 通过 | transport、Web RPC 映射、Tauri 注册和公开范围断言 |
| 浏览器回归 | 1/1 通过 | `/`、`/sessions`、`/labcontext` 的 mock Web RPC 流程 |
| Rust 工作区测试 | 通过 | `cargo test --workspace --locked`；服务库 933 项测试通过 |
| Tauri 库测试 | 49/49 通过 | `cargo test --manifest-path apps/src-tauri/Cargo.toml --locked --lib` |
| 插件结构验证 | 通过 | `validate_plugin.py plugins/codexmanager-connector` |
| 公开发布预检 | 通过 | 秘密/运行时文件、个人插件连接、部署引用和 Rust manifest 检查 |
| Shell、Python、YAML 检查 | 通过 | 发布脚本、Python 编译和 Compose/Workflow YAML 可解析 |
| 源码包生成与清单 | 通过 | 只包含源码；无数据库、token、`.env`、构建缓存、Tauri `gen` 或原始交付包 |

## 已验证的公开范围

- Web UI 维护三个顶级路由：`/`、`/sessions` 和 `/labcontext`。
- 公共 Web RPC 与 Tauri invoke 表只保留服务端仍支持的能力；历史插件市场、账号导入导出和 warmup 命令没有公开注册。
- LabContext 管理地址限制为本地 loopback 或 Docker 宿主机网关，不能借由该配置连接任意远程管理端。
- 桌面更新需要显式 `CODEXMANAGER_UPDATE_REPO=owner/repository`，没有默认上游更新源。
- Codex 插件模板是 skills-only；本项目当前不声明 MCP 服务或用户专属连接。

## 尚未在此主机执行的外部验证

- Docker CLI / Docker Compose 不可用，因此 `preflight.sh` 已验证其余门禁，但跳过了 Compose `config` 渲染和真实容器 smoke test。
- 本机未安装 gitleaks 或 trufflehog。仓库内预检覆盖常见文件名与秘密模式；在任何公开推送前，仍须用专用工具扫描工作树与最终 Git 历史。
- GitHub 仓库 URL、维护者资料、公开服务域名、隐私政策和服务条款属于维护者决定，未填入源码或模板。

这些未执行项不是已知测试失败；它们依赖目标 GitHub 仓库或具备 Docker/秘密扫描器的环境。
