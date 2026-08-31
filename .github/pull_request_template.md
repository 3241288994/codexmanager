## 变更摘要

- <!-- 请填写 -->

## 改动范围

- [ ] Frontend
- [ ] Desktop / Tauri
- [ ] Service
- [ ] Gateway / Protocol Adapter
- [ ] Docs / Governance
- [ ] Workflow / Release

## 主要文件

- <!-- 请填写 -->

## 验证

- [ ] `corepack pnpm@10.30.3 -C apps run lint`
- [ ] `corepack pnpm@10.30.3 -C apps run build:desktop`
- [ ] `corepack pnpm@10.30.3 -C apps run test:runtime`
- [ ] `corepack pnpm@10.30.3 -C apps run test:e2e`（路由/UI 改动）
- [ ] `cargo test --workspace --locked -- --test-threads=1`
- [ ] `cargo test --manifest-path apps/src-tauri/Cargo.toml --locked --lib`（Tauri 改动）
- [ ] `scripts/open-source/preflight.sh`（发布、部署或公开配置改动）
- [ ] 其他本地验证已说明

已执行的实际验证：

```text

```

未执行的验证与原因：

```text

```

## 风险与影响面

- <!-- 请填写 -->

## 备注

- 提交前请确认未包含敏感 token、cookie、API key
