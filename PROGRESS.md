# PROGRESS.md

最后更新：2026-04-13

## 当前状态

项目已完成用户/认证/签到核心链路、4 种独立内容生成资源的异步任务架构、完整的学习主题模块，以及管理员充值接口。

当前可通过 `cargo check` 和 `cargo test`（89 个测试全部通过）。

## 已完成

- 搭建 `axum + tokio + sea-orm` 基础框架。
- 配置 `sea-orm` 同时支持：
  - `sqlite`
  - `postgresql`
  - `mysql`
- 接入 `argon2` 密码哈希。
- 实现统一错误模型：
  - 业务错误使用枚举
  - 校验错误统一返回 `VALIDATION_FAILED`
  - 对外错误文案默认中文
- 完成基础路由：
  - `POST /api/v1/users`
  - `POST /api/v1/tokens`
  - `GET /api/v1/me`
  - `PATCH /api/v1/me`
  - `POST /api/v1/checkins`
  - `GET /api/v1/checkins`
  - `GET /health`
- 已接入 SeaORM migration，启动时会自动执行迁移。
- 当前 migration 采用早期开发阶段的顺序编号风格，首个 migration 为 `m0001_init_schema.rs`。
- 已补充基础 HTTP 级测试，覆盖认证、签到、内容生成回调等场景。

### 内容生成模块

- 实现 4 种独立内容生成资源：
  - `knowledge_video`（知识点视频，扣钻石）
  - `code_video`（代码讲解视频，扣钻石）
  - `interactive_html`（交互式网页，扣金币）
  - `knowledge_explanation`（知识点文字讲解 + 思维导图，扣金币）
- 每种资源各自定义独立状态枚举，当前状态：QUEUING / GENERATING / FINISHED / FAILED
- 用户端接口：
  - `POST /api/v1/{resource}` — 创建生成任务（扣费 + 向微服务发请求 + 等待入队确认）
  - `GET /api/v1/{resource}/{id}` — 查询任务状态（短轮询）
  - `PATCH /api/v1/{resource}/{id}` — 设置公开 / 对 FAILED 任务重新生成
- 微服务回调接口（internal，通过 API_KEY 鉴权）：
  - `PATCH /api/v1/internal/{resource}/{id}` — 更新任务状态
  - 状态流转校验：QUEUING→GENERATING, GENERATING→FINISHED/FAILED
  - FAILED 时自动退款到用户账户
- 新增 `ServiceAuth` 提取器：通过 `sk-` 前缀区分 API_KEY 和 JWT
- 每种微服务各自独立的 URL、API_KEY、费用配置（环境变量）
- `knowledge_explanation` 直接存储 `content`（文本）和 `mindmap`（JSON），不使用 url
- 其余三种资源使用 `url` 字段（由微服务回调写入）
- `knowledge_explanation` 新增 `cost` 字段区分付费（用户自主生成）和免费（学习任务下生成）
- 已补充集成测试：回调状态更新、退款、非法状态流转、错误 API_KEY、用户 PATCH 等

### 学习主题模块

- 实现完整学习主题生命周期，新增 7 个实体：
  - `study_subject`（8 个状态的状态机）
  - `problem`（题目，归属用户，单选 A/B/C/D）
  - `pretest_problem`（课前测关联表，含自信程度）
  - `study_stage`（学习阶段，3 状态顺序解锁）
  - `study_task`（学习任务，3 状态顺序解锁，持有 3 个可空内容外键）
  - `study_quiz`（小测，5 状态，含免费额度限制）
  - `study_quiz_problem`（小测题目关联表）
- 课前测生成 → 用户逐题作答 → 学习计划结构生成 → 阶段/任务顺序解锁 → 任务内容按需生成 → 小测生成/作答/提交 → 题目收藏/错题查询
- 3 个新增微服务 dispatch 函数（pretest / plan / quiz），POST 到根路径
- 8×6 回调状态转换表（10 个合法转换，38 个拒绝）
- 顺序解锁逻辑：完成任务 → 解锁下一个任务 → 阶段完成 → 解锁下一个阶段
- 小测免费额度机制：每任务 N 次免费（可配置），超出扣金币
- 所有权校验通过 JOIN 链追溯到 `study_subject.user_id`

## 当前签到规则实现

- 只保留 `POST /checkins`，不再提供单独的 `/checkins/makeup`。
- 请求体支持 `makeup?: boolean`。
- 若上次签到后发生断签，用户只能在本次签到时决定是否一次性补签。
- 若选择补签：
  - 会补上上次断签后的全部日期
  - 会发放从断签前连续签到天数之后开始，到补签完成当天为止的全部签到奖励
  - 金币成本按断签天数计算
  - 钻石成本固定一次扣除
  - 若金币或钻石不足，则本次签到失败，不自动降级为普通签到
- 签到金币奖励使用可配置序列：
  - 默认 `1,2,3,4,6,8,10`
  - 超出序列长度后沿用最后一个值
- 签到当前不发经验。

### 充值接口

- 实现管理员充值接口 `POST /internal/users/{id}/balance`，通过 API_KEY 鉴权（`ServiceKind::Recharge`）。
- 支持同时增减金币和钻石，正数增加、负数扣减，扣减后余额不能为负。
- 新增环境变量 `RECHARGE_API_KEY`（默认 `sk-recharge-dev`）。
- 已补充 10 个集成测试覆盖：正常充值、仅充金币、扣减、余额不足、用户不存在、空请求体、错误 API_KEY、其他服务 Key 被拒、多次累积。

## 尚未完成

- 聚合查询接口（`my-contents`、`public-contents`）
- 更多业务表结构与后续 schema 演进

## 下一步建议

1. 实现 `my-contents` 和 `public-contents` 聚合查询。
2. 补充学习主题模块的集成测试。
3. 在新增业务资源时同步补对应 migration；在仍可删库重建的阶段内，优先维护初始化 migration 的清晰度。