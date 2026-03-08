# Relay — 本地 Coding Agent 账号编排器（CLI-first / macOS UI）V1 开发文档

## 1. 项目定义

### 1.1 项目名称

**Relay**

含义：
- 表达“接力 / 切换 / 续航”，对应多个账号之间的顺滑接替。
- 不绑定任何特定 Agent 品牌。
- 适合同时承载 CLI 内核与桌面 UI。
- 后续可扩展为多 Agent 编排层，而不需要改名。

一句话定位：

> Relay 是一个以 CLI 为核心的本地账号编排工具，用于在多个 Coding Agent 账号之间安全切换；V1 先支持 Codex，提供跨平台 CLI，并在 macOS 15+ 上提供原生菜单栏 UI。

### 1.2 文档目标

本文档用于指导 Coding Agent 开发 V1 产品，要求：
- **核心能力先做成 CLI 工具**
- CLI 具备未来扩展到 Linux 的能力
- **macOS 15+ 提供原生菜单栏 UI**
- 当前只实现 **Codex** 适配
- 但架构上必须为未来支持更多 Agent 预留接口

---

## 2. 产品策略

### 2.1 总体路线

采用双层架构：

1. **Relay Core CLI**
   - 负责账号管理、切换事务、健康检查、自动切换、日志、配置读写
   - 作为系统真实执行层
   - 可运行在 macOS / Linux

2. **Relay for macOS**
   - 负责菜单栏交互、设置页、状态展示、通知、开机启动
   - 不直接实现切换逻辑
   - 通过调用 Relay CLI 完成所有真实操作

### 2.2 为什么这样设计

原因：
- 真正跨平台的是“账号编排能力”，不是菜单栏 UI。
- CLI-first 可以更容易调试、自动化测试、适配 Linux。
- macOS UI 只作为控制面板和状态展示层，复杂度更低。
- 后续即使增加 Linux GUI、Raycast 插件、Web 控制台，也都可以复用 CLI 内核。

### 2.3 V1 聚焦范围

V1 只做：
- Codex 账号管理与切换
- 跨平台 CLI（macOS / Linux 设计兼容，优先在 macOS 完成）
- macOS 原生菜单栏 UI
- 安全回滚
- 基于明确失败信号的自动切换

V1 不做：
- 多 Agent 正式支持
- 复杂额度看板
- 浏览器 Cookie 抓取
- 私有接口逆向
- 云同步
- Windows

---

## 3. 用户故事

### 3.1 CLI 用户故事

1. 用户可以通过命令行添加、列出、启用、禁用、删除账号配置。
2. 用户可以查看当前生效账号。
3. 用户可以执行一次手动切换。
4. 用户可以启用自动切换策略。
5. 当当前账号触发明确失败时，CLI 可以切到下一个可用账号。
6. 用户可以导出诊断信息。

### 3.2 macOS UI 用户故事

1. 用户安装后可在菜单栏看到当前状态。
2. 用户可以从菜单栏快速切换账号。
3. 用户可以打开设置页管理账号与策略。
4. 用户可以看到最近一次切换结果。
5. 用户可以开启开机启动。

---

## 4. 产品边界与原则

### 4.1 核心原则

- **CLI 是唯一真实执行入口**
- UI 只负责调用 CLI 和展示结果
- 切换行为必须 **事务化、可验证、可回滚**
- 默认只操作 **用户级 Codex 配置**，不改项目内 `.codex/`
- 敏感信息不明文入库
- 自动切换必须可关闭
- 自动切换只基于明确、可验证的失败信号

### 4.2 合规边界

V1 不以“绕过额度限制”为产品卖点。

产品表述应为：
- 多账号本地管理
- 安全切换
- 故障转移
- 本地配置编排

避免：
- 抓取浏览器登录态作为主路径
- 伪造登录
- 逆向未公开协议
- 规避平台限制作为宣传点

---

## 5. 技术栈建议

### 5.1 Relay Core CLI

建议技术栈：
- 语言：**Rust**
- CLI 框架：`clap`
- 序列化：`serde`, `serde_json`, `toml`
- 数据库存储：`rusqlite`
- 日志：`tracing`, `tracing-subscriber`
- Keychain / Secret：
  - macOS：Security framework / Keychain 封装
  - Linux：先做文件引用 + 环境变量模式，后续可接 Secret Service
- 文件系统监听：`notify`
- 原子写入：自实现事务写入工具层
- 错误处理：`thiserror`, `anyhow`
- 测试：`cargo test`

选择 Rust 的原因：
- 更适合作为长期演进的跨平台系统工具
- 对文件系统、事务写入、CLI、守护进程式状态管理更稳
- 后续如果 UI 需要更深绑定，也可以通过 JSON CLI / 本地 IPC 与 Swift 通信

### 5.2 macOS UI

建议技术栈：
- 语言：Swift 6
- UI：SwiftUI
- 菜单栏：`MenuBarExtra` 为主，必要时混用 AppKit
- 设置窗口：SwiftUI
- 开机启动：`SMAppService`
- 调用 CLI：`Process`
- 日志读取：读取 Relay CLI 的结构化日志/状态文件

### 5.3 CLI 与 UI 通信方式

V1 使用：
- **标准输入输出 JSON 模式**
- **状态文件 + 日志文件**

不做：
- 自定义 socket daemon
- XPC
- gRPC

原因：
- 最简单
- 最可调试
- 最利于 Linux 复用

---

## 6. 架构设计

### 6.1 总体架构

```text
+-----------------------+
|   Relay macOS UI      |
|  (SwiftUI/MenuBar)    |
+-----------+-----------+
            |
            | Process + JSON
            v
+-----------------------+
|    Relay Core CLI     |
|      (Rust)           |
+-----------+-----------+
            |
            +-------------------+
            |                   |
            v                   v
+------------------+   +------------------+
| Codex Adapter    |   | Local Store      |
| config/auth/etc. |   | sqlite/json/logs |
+------------------+   +------------------+
```

### 6.2 核心模块

#### Relay CLI App
负责命令解析、输出格式、错误码、调用服务层。

#### Core Services
- `profile_service`
- `switch_service`
- `policy_service`
- `diagnostics_service`
- `health_service`

#### Adapter Layer
- `agent_adapter` trait
- `codex_adapter` implementation

#### Storage Layer
- `sqlite_store`
- `snapshot_store`
- `secret_store`
- `state_store`

#### Platform Layer
- `platform_macos`
- `platform_linux`

---

## 7. 目录结构

```text
relay/
  apps/
    relay-cli/
    relay-macos/
  crates/
    relay-core/
    relay-adapters/
    relay-store/
    relay-platform/
    relay-types/
  docs/
  scripts/
```

更细化建议：

```text
apps/
  relay-cli/
    src/main.rs
  relay-macos/
    RelayApp.xcodeproj
    RelayApp/
      App/
      MenuBar/
      Settings/
      Services/
      Models/
      Resources/

crates/
  relay-types/
    src/
      profile.rs
      state.rs
      events.rs
      errors.rs
      protocol.rs

  relay-core/
    src/
      services/
        profile_service.rs
        switch_service.rs
        policy_service.rs
        health_service.rs
        diagnostics_service.rs
      transaction/
      validation/

  relay-adapters/
    src/
      agent_adapter.rs
      codex/
        mod.rs
        config.rs
        auth.rs
        detect.rs
        validate.rs
        switch.rs

  relay-store/
    src/
      sqlite/
      snapshots/
      secrets/
      state/

  relay-platform/
    src/
      macos/
      linux/
      process/
      filesystem/
      atomic_write/
```

---

## 8. 数据与状态模型

### 8.1 Profile

```rust
pub struct Profile {
    pub id: String,
    pub nickname: String,
    pub agent: AgentKind,
    pub priority: i32,
    pub enabled: bool,
    pub codex_home: Option<String>,
    pub config_path: Option<String>,
    pub auth_mode: AuthMode,
    pub metadata: serde_json::Value,
}
```

V1 要点：
- `agent` 目前仅支持 `Codex`
- `codex_home` 用于未来隔离 profile
- `metadata` 预留扩展字段

### 8.2 Active State

```rust
pub struct ActiveState {
    pub active_profile_id: Option<String>,
    pub last_switch_at: Option<String>,
    pub last_switch_result: SwitchResult,
    pub auto_switch_enabled: bool,
}
```

### 8.3 Switch Checkpoint

```rust
pub struct SwitchCheckpoint {
    pub checkpoint_id: String,
    pub backup_paths: Vec<String>,
    pub created_at: String,
}
```

### 8.4 Failure Event

```rust
pub enum FailureReason {
    AuthInvalid,
    QuotaExhausted,
    RateLimited,
    CommandFailed,
    ValidationFailed,
    Unknown,
}
```

---

## 9. Codex 适配策略（V1）

### 9.1 V1 真实目标

V1 不追求“完整理解所有 Codex 内部状态”，只做以下几件可靠的事：
- 发现 Codex CLI 是否安装
- 识别用户级配置位置
- 管理多个本地账号配置档案
- 切换当前激活账号
- 做切换后验证
- 在失败时回滚

### 9.2 账号表示方式

V1 中，一个账号配置不是“在线登录流程本身”，而是一个本地 **Profile**：
- 账号昵称
- 关联的 Codex 配置目录或凭据引用
- 优先级
- 启用/禁用状态
- 认证模式说明

### 9.3 推荐切换策略

V1 推荐采用：
- **Profile 目录切换** 或
- **用户级配置文件切换**

避免：
- 直接操纵项目目录下 `.codex/`
- 强依赖浏览器会话
- 隐式改动用户未知文件

### 9.4 激活流程

激活一个 Profile 时：

1. 检查目标 Profile 是否可用
2. 快照当前 live 配置
3. 写入临时配置
4. 原子替换 live 配置
5. 执行验证命令
6. 成功则提交
7. 失败则自动回滚

### 9.5 验证机制

验证分为两类：
- **静态验证**：配置存在、文件完整、路径合法
- **动态验证**：调用一条轻量 Codex 命令，确认当前环境可用

V1 必须支持：
- 切换后运行验证
- 验证失败自动回滚

---

## 10. CLI 设计

### 10.1 命令原则

- 命令要兼容脚本调用
- 默认输出人类可读文本
- 提供 `--json` 供 UI 调用
- 错误码稳定

### 10.2 命令草案

```bash
relay doctor
relay status
relay profiles list
relay profiles add
relay profiles edit <id>
relay profiles remove <id>
relay profiles enable <id>
relay profiles disable <id>
relay profiles import-codex
relay switch <id>
relay switch next
relay auto-switch enable
relay auto-switch disable
relay events list
relay logs tail
relay diagnostics export
```

### 10.3 JSON 输出要求

所有给 UI 调用的命令都必须支持：

```bash
relay status --json
relay profiles list --json
relay switch <id> --json
```

输出要求：
- 稳定 schema
- 包含 `success`
- 包含 `error_code`
- 包含 `message`
- 包含业务 payload

示例：

```json
{
  "success": true,
  "message": "switch completed",
  "data": {
    "active_profile_id": "p_123",
    "switched_at": "2026-03-07T10:00:00Z"
  }
}
```

---

## 11. macOS UI 设计

### 11.1 UI 定位

macOS UI 不是主执行引擎，而是：
- 菜单栏控制器
- 设置面板
- 状态可视化层

### 11.2 菜单栏菜单项（V1）

- 当前状态（已激活账号 / 不可用 / 未配置）
- 当前账号昵称
- 一键切换到下一个账号
- 账号列表
- 自动切换开关
- 打开设置
- 导出诊断
- 退出

### 11.3 设置页模块

#### General
- 开机启动
- 自动切换开关
- 日志级别

#### Profiles
- 账号列表
- 新增/编辑/删除
- 优先级调整
- 启用/禁用
- 导入已有 Codex 配置

#### Activity
- 最近切换记录
- 最近错误记录

#### Diagnostics
- 配置检测
- 日志导出
- 版本信息

### 11.4 UI 与 CLI 协作规则

- UI 不直接改写 Codex 文件
- UI 所有操作都调用 `relay` 命令
- UI 只解析标准 JSON 输出
- UI 只展示 CLI 的已知错误码和消息

---

## 12. 自动切换策略（V1）

### 12.1 触发原则

V1 不做复杂预测式切换，只做：
- 明确失败后切换
- 用户手动切换

### 12.2 允许的触发原因

- 鉴权失败
- 额度耗尽/不可继续
- 明确 rate limit
- 健康检查失败
- 用户手动请求“切到下一个”

### 12.3 账号选择策略

默认策略：
- 只从 `enabled = true` 的 Profile 中选择
- 按 priority 升序或降序（实现时固定一种并在 UI 解释）
- 跳过最近失败且处于 cooldown 的 Profile
- 找到第一个可用账号即停止

### 12.4 回退行为

若没有可用账号：
- 保持当前状态不变
- 返回明确错误
- 记录事件
- UI 发通知

---

## 13. 存储设计

### 13.1 配置目录

建议：

```text
~/.relay/
  relay.db
  logs/
  state.json
  snapshots/
  exports/
```

### 13.2 存什么

SQLite：
- profiles
- switch_history
- failure_events
- app_settings

JSON：
- 当前状态缓存
- UI 快速读取状态

Keychain / Secret Store：
- 敏感 token 引用
- 密钥材料

### 13.3 不存什么

- 明文账号密码
- 明文长效 token（若平台可避免）
- 项目私有配置副本

---

## 14. 事务与回滚

### 14.1 切换事务要求

每次切换必须实现：
- 创建 checkpoint
- 备份 live 文件
- 临时写入
- 原子替换
- 验证
- 成功提交或失败回滚

### 14.2 原子写入流程

```text
read current -> backup -> write temp -> fsync -> rename -> validate
```

### 14.3 回滚流程

```text
validation failed -> restore backup -> revalidate old state -> mark failure
```

### 14.4 失败时要求

- 不允许留下半写入状态
- 必须记录明确错误原因
- 必须向 UI 返回结构化错误

---

## 15. 日志与诊断

### 15.1 日志级别

- error
- warn
- info
- debug

### 15.2 日志内容

记录：
- 切换开始/成功/失败
- 验证结果
- 自动切换触发原因
- 文件备份与回滚动作

避免记录：
- 明文密钥
- 明文 token
- 完整敏感路径内容

### 15.3 诊断导出

`relay diagnostics export` 生成 zip：
- 版本信息
- 环境信息
- 脱敏配置摘要
- 最近日志
- 最近事件

---

## 16. 开发阶段计划

### Phase 0：项目初始化

目标：
- 建立 monorepo
- 建立 Rust workspace
- 建立 macOS app skeleton
- 建立基础 CI

任务：
- 初始化 `relay-cli`
- 初始化 `relay-macos`
- 建立共享协议文档
- 建立编码规范与错误码规范

验收标准：
- CLI 可运行 `relay --help`
- macOS App 可启动并显示菜单栏图标

---

### Phase 1：Codex 检测与 Profile 管理

目标：
- 能检测 Codex 安装
- 能管理本地 profile

任务：
- 实现 `relay doctor`
- 实现 `profiles list/add/remove/enable/disable`
- 实现 SQLite profile store
- 实现基础 JSON 输出

验收标准：
- CLI 能完整管理 profile
- `relay doctor` 能输出可读诊断和 JSON 诊断

---

### Phase 2：切换事务与回滚

目标：
- 实现可靠切换
- 实现 checkpoint 与回滚

任务：
- 实现 `codex_adapter.activate()`
- 实现快照目录
- 实现原子写入工具
- 实现切换后验证
- 实现失败回滚
- 实现 `relay switch <id>` 与 `relay switch next`

验收标准：
- 成功切换后 active state 更新
- 验证失败会自动回滚
- 任意失败不破坏原有配置

---

### Phase 3：自动切换与事件记录

目标：
- 能根据明确失败信号自动切换

任务：
- 实现 failure event 模型
- 实现 cooldown 机制
- 实现 `auto-switch enable/disable`
- 实现事件记录和查询

验收标准：
- 提供事件列表
- 明确失败时可自动选中下一个可用账号

---

### Phase 4：macOS 菜单栏 UI

目标：
- 提供完整的 macOS 原生控制面板

任务：
- 菜单栏状态展示
- 当前账号显示
- 账号列表与手动切换
- 自动切换开关
- 设置页
- 活动日志页
- 通过 `Process` 调用 CLI

验收标准：
- UI 不直接操作底层文件
- 所有功能均通过 CLI 完成
- 菜单栏可完成主要操作闭环

---

### Phase 5：稳定性与发布

目标：
- 准备首个可分发版本

任务：
- 开机启动
- 错误提示优化
- 诊断导出
- 签名、notarization
- 打包流程

验收标准：
- 本地可安装
- 首次启动可完成引导
- 出错时可导出诊断包

---

## 17. Coding Agent 执行顺序建议

Coding Agent 应按以下顺序开发：

1. 建 monorepo 与 workspace
2. 定义 Rust 类型与 JSON 协议
3. 实现 profile store
4. 实现 Codex 检测
5. 实现切换事务
6. 实现验证和回滚
7. 实现自动切换
8. 实现日志和诊断
9. 最后再做 macOS UI

重要：
- **不要先做 UI 再补核心逻辑**
- 所有 UI 功能都必须建立在 CLI 已可用基础上

---

## 18. MVP 验收清单

V1 MVP 达成条件：
- 可以安装并运行 `relay` CLI
- 可以添加多个 Codex profile
- 可以查看当前 active profile
- 可以手动切换 profile
- 切换失败会回滚
- 可以启用/禁用自动切换
- 可以导出日志和诊断
- macOS 菜单栏 UI 可以调用上述核心能力

---

## 19. 后续扩展预留

V2 可扩展方向：
- Linux TUI 或桌面壳
- 多 Agent adapter
- 更细粒度额度状态
- Provider 级健康探测
- 更丰富的通知策略
- 背景守护模式
- 导入向导

---

## 20. Coding Agent 工作约束

在实现过程中，Coding Agent 必须遵守：
1. 所有真实切换逻辑写在 CLI 内核，不写在 UI 中。
2. 所有文件改写必须经过事务化写入与备份。
3. 所有对外输出命令必须支持 `--json`。
4. 所有错误都要有稳定错误码。
5. 所有敏感信息必须脱敏。
6. 不要改动项目目录内 `.codex/`。
7. 先实现 Codex，禁止在 V1 中扩散到其他 Agent。
8. 在没有验证机制之前，不允许实现自动切换。

---

## 21. 首次实现优先级

### P0
- CLI 框架
- profile store
- Codex 检测
- 手动切换
- 回滚

### P1
- 自动切换
- 事件记录
- 诊断导出
- macOS 菜单栏主流程

### P2
- 设置页完善
- 开机启动
- 导入已有配置
- 细化错误提示

---

## 22. 一句话交付要求

> 先交付一个可靠的 Relay CLI，再让 macOS 菜单栏 UI 作为这个 CLI 的原生控制面板。V1 只支持 Codex，但所有接口和数据模型必须为未来多 Agent 扩展留出空间。

