# AGENTS.md

本文件面向参与本仓库工作的 agent 与开发者，描述长期有效的协作约定。

## 文件分工

- `AGENTS.md`：记录稳定的项目协作规则、目录约定、设计边界、实现原则。
- `PROGRESS.md`：记录当前阶段的实现进度、已完成事项、待办、临时决策。

不要把强依赖当前进度的信息长期堆积在 `AGENTS.md` 中。

## 项目目标

- 本项目是 `zhiying-tutor` 的后端服务。
- 技术栈以 Rust 为主，使用 `axum + tokio + sea-orm`。
- 密码哈希使用 `argon2`。
- 数据库层要求兼容多个主流数据库，当前目标是：
  - `sqlite`
  - `postgresql`
  - `mysql`

## 设计原则

- 对外 API 优先按资源领域建模，而不是按“认证模块”“用户模块”这类宽泛业务分组。
- 新接口优先沿用 REST 风格资源命名。
- 错误响应统一走后端错误模型，不直接把底层英文错误暴露给前端。
- 面向中文用户，用户可见错误信息默认使用中文。
- 校验错误统一返回 `VALIDATION_FAILED`，不要直接透传 `validator` 的原始英文结构。

## 当前目录约定

- `src/routes/`：按资源领域拆分的 HTTP handler 与路由。
- `src/entities/`：SeaORM entity 定义。`entities/common.rs` 存放跨实体共享的枚举（如 `ProblemAnswer`）。
- `src/services/`：不直接依赖 HTTP 协议的业务逻辑或通用逻辑，包括微服务 dispatch 函数。
- `src/migration/`：SeaORM migration 定义。
- `src/config.rs`：环境变量配置解析。
- `src/error.rs`：统一错误码、错误响应与错误映射。
- `src/auth.rs`：JWT 鉴权（`AuthUser`）和微服务 API Key 鉴权（`ServiceAuth` / `ServiceKind`）。
- `src/main.rs`：应用启动入口，启动时直接执行 migration。

## 路由约定

- 资源路径与模块命名尽量保持一致。
- 当前用户资源统一使用 `me`，不要再引入 `self` 命名。
- 新增路由时，优先新增同名资源文件，而不是继续往已有无关模块中堆积。

## 命名约定

- 表示数量、计数、累计值的字段统一使用复数名词，例如 `total_checkins`、`streak_checkins`、`total_stages`、`finished_tasks`。不要使用单数形式（如 `total_checkin`）作为计数字段名。
- 该规则同时适用于实体字段、API 响应字段、测试断言中的 JSON key，以保持端到端一致。

## 数据库约定

- 不要把实现写死在 PostgreSQL 方言上。
- 涉及 schema 演进时，优先使用 SeaORM migration 维护表结构。
- 当前仍处于早期开发阶段，可接受删库重建，不强求保留完整历史迁移兼容路径。
- 在进入需要兼容旧库的数据阶段前，migration 文件优先使用顺序编号命名，例如 `m0001_init_schema.rs`，不要默认使用日期时间戳命名。
- 早期若表结构调整幅度较大，可以直接改写当前初始化 migration，并通过删库重建验证；等进入需要保留升级路径的阶段后，再切换为只追加新 migration。

## 配置约定

- 可调业务参数优先放入环境变量，而不是散落在代码常量中。
- 新增配置项时，同步更新：
  - `src/config.rs`
  - `.env.example`
  - 必要的接口文档

## 文档约定

- 若只是当前阶段是否已实现、做到哪一步、下一步做什么，这类内容应写入 `PROGRESS.md`，不应写入 `AGENTS.md`。

## 开发约定

- 新增依赖优先使用 `cargo add`。
- 修改后至少执行：
  - `cargo fmt`
  - `cargo check`
- 当前仍处于早期开发阶段，不需要为了兼容旧实现而保留无实际价值的老旧代码、过渡封装或未再使用的旧文件；确认无引用后应直接删除。
- 提交前先参考 `git log --oneline` 的现有风格，当前提交信息使用简洁的前缀式格式，例如 `feat: ...`、`init: ...`。
- 如果某个模块尚未实现，优先提供清晰的占位路由或明确的未实现错误，而不是留下行为不明的半成品。

## 异步微服务回调约定

- 内容生成类微服务（knowledge_video / code_video / interactive_html / knowledge_explanation）的回调使用 `PATCH /internal/{resource}/{id}`，dispatch 发送到 `{SERVICE_URL}/generate`。
- 学习主题类微服务（pretest / plan / quiz）的回调使用 `POST /internal/{resource}/{id}`，dispatch 发送到 `{SERVICE_URL}` 根路径。
- 微服务 API Key 统一以 `sk-` 前缀开头，通过 `ServiceAuth` 提取器区分来源。
- 回调状态枚举统一为大写字符串：`QUEUING`、`GENERATING`、`FINISHED`、`FAILED`。
- FAILED 状态统一触发退款，退款金额读取记录上的 `cost` 字段（而非配置值）。

## 学习主题模块设计约定

- 学习主题状态机为 8 个状态（PretestQueuing → ... → Studying → Finished / Failed），回调通过 API Key 区分 pretest 和 plan 来源。
- 阶段（StudyStage）和任务（StudyTask）各 3 个状态（Locked / Studying / Finished），强制按 sort_order 顺序解锁。
- 资源所有权校验通过 JOIN 链向上追溯到 `study_subject.user_id`。
- 题目（problem）归属用户，课前测（pretest_problem）和小测（study_quiz_problem）为关联表。
- 统计字段（total_stages/finished_stages、total_tasks/finished_tasks）使用非规范化设计，在状态变更时维护。
