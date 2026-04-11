# PROGRESS.md

最后更新：2026-04-02

## 当前状态

项目已完成用户/认证/签到核心链路，以及 4 种独立内容生成资源的异步任务架构。

当前可通过 `cargo check` 和 `cargo test`（12 个测试全部通过）。

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

### 内容生成模块（新增）

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
- 已补充集成测试：回调状态更新、退款、非法状态流转、错误 API_KEY、用户 PATCH 等

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

## 当前配置项

- `APP_HOST`
- `APP_PORT`
- `DATABASE_URL`
- `JWT_SECRET`
- `JWT_TTL_DAYS`
- `CORS_ALLOW_ORIGIN`
- `CHECKIN_REWARD_SEQUENCE`
- `CHECKIN_MAKEUP_GOLD_COST_PER_DAY`
- `CHECKIN_MAKEUP_DIAMOND_COST`
- `KNOWLEDGE_VIDEO_DIAMOND_COST`
- `CODE_VIDEO_DIAMOND_COST`
- `INTERACTIVE_HTML_GOLD_COST`
- `KNOWLEDGE_EXPLANATION_GOLD_COST`
- `KNOWLEDGE_VIDEO_SERVICE_URL` / `KNOWLEDGE_VIDEO_API_KEY`
- `CODE_VIDEO_SERVICE_URL` / `CODE_VIDEO_API_KEY`
- `INTERACTIVE_HTML_SERVICE_URL` / `INTERACTIVE_HTML_API_KEY`
- `KNOWLEDGE_EXPLANATION_SERVICE_URL` / `KNOWLEDGE_EXPLANATION_API_KEY`

## 尚未完成

- 学习计划相关实体与接口（含课前测 + 学习计划生成异步流程）
- 学习阶段相关实体与接口
- 学习任务相关实体与接口
- 题目与前测相关实体与接口
- 学习计划关联的 knowledge_explanation（独立表，区别于用户自主生成）
- 聚合查询接口（`my-contents`、`public-contents`）
- 更多业务表结构与后续 schema 演进

## 下一步建议

1. 按资源领域优先落地 `study_plans`（含课前测 + 异步生成流程），再逐步展开 `study_stages / study_tasks / problems`。
2. 实现 `my-contents` 和 `public-contents` 聚合查询。
3. 在新增业务资源时同步补对应 migration；在仍可删库重建的阶段内，优先维护初始化 migration 的清晰度。
