# PROGRESS.md

最后更新：2026-05-11

## 当前状态

核心学习闭环（学前测 → 计划 → 阶段/任务解锁 → 任务内容生成 → 小测 → 错题/收藏）已通；三大内容资源（K2V / C2V / Interactive HTML）已支持工具自由创建（list/get/delete via link 表）；微服务 dispatch 走 RabbitMQ + publisher confirm，callback 仍走 HTTP。

## 已完成

### 基础设施
- SeaORM migration 单文件 `m0001_init_schema.rs`，启动时直接 apply（早期阶段允许删库重建）。
- 数据库兼容 `sqlite / postgresql / mysql`，本地默认 sqlite，联调默认 PostgreSQL（`zhiying-infra` 提供）。
- 错误统一走 `src/error.rs`：业务错误中文化、校验错误归并为 `VALIDATION_FAILED`。
- 鉴权：JWT（`AuthUser`） + 微服务 API Key（`ServiceAuth` / `ServiceKind`，`sk-` 前缀区分来源）。
- CORS、TraceLayer、`/health`。

### 用户体系
- `POST /users` 注册（赠送 `REGISTER_BONUS_DIAMONDS`）。
- `POST /tokens` 登录签发 JWT。
- `GET /me`、`PATCH /me`、`PATCH /me/username`（用户名唯一约束 + 友好错误）。
- `GET /me/mistakes`、`GET /me/bookmarks`：错题本与收藏聚合。
- 签到：`POST /checkins`、`GET /checkins`，奖励序列由 `CHECKIN_REWARD_SEQUENCE` 配置；钻石/金币补签。
- 管理员充值：`POST /internal/users/{id}/balance`（`ServiceKind::Recharge`）。

### 学习主题
- `study_subject` 8 状态状态机（PretestQueuing → PretestGenerating → PretestReady → PlanQueuing → PlanGenerating → Studying → Finished / Failed）。
- 阶段 / 任务 3 状态（Locked / Studying / Finished），按 `sort_order` 强制顺序解锁。
- 统计字段非规范化（`total_stages / finished_stages / total_tasks / finished_tasks`），状态变更时同步维护。
- 接口：
  - `POST /study-subjects` 创建（按 `total_stages → 钻石消耗`配置扣费）+ dispatch pretest。
  - `GET /study-subjects`、`GET /study-subjects/{id}`、`GET /study-subjects/{id}/stages`（含 tasks）。
  - `GET/PATCH /study-subjects/{id}/pretest[/problem_id]`：学前测做题。
  - `POST /study-subjects/{id}/plan`：触发计划生成。
  - `GET /study-stages/{id}`、`GET /study-tasks/{id}`、`POST /study-tasks/{id}/complete`。
  - 任务派生内容：`POST/GET /study-tasks/{id}/knowledge-video|interactive-html`、`POST /study-tasks/{id}/explanation`。
  - 小测：`POST /study-tasks/{id}/quizzes`、`GET /study-tasks/{id}/quizzes`、`GET /study-quizzes/{id}`、`PATCH /study-quizzes/{quiz_id}/problems/{study_quiz_problem_id}`、`POST /study-quizzes/{id}/submit`。
  - 错题/收藏切换：`PATCH /quiz-problems/{id}/bookmark`、`PATCH /quiz-problems/{id}/mistake-visibility`。
- 用户主动切换 active subject：`PATCH /me` 写 `active_study_subject_id`。

### 内容生成（工具自由创建 + 任务派生双路径）
- 三件套统一形态：`POST /<resource>`（扣费 + dispatch + INSERT link）、`GET /<resource>`（list via link）、`GET /<resource>/{id}`（鉴权走 link）、`DELETE /<resource>/{id}`（仅删 link，资源行保留）。
  - `knowledge-videos`、`code-videos`、`interactive-htmls`。
- `knowledge-explanations` 暂只支持任务派生，未提供自由创建。
- ownership 拆分到 `user_<resource>_link` 三张 join 表；任务派生路径走 `study_task → study_stage → study_subject.user_id`。
- 状态枚举统一大写：QUEUING / GENERATING / FINISHED / FAILED。FAILED 按记录 `cost` 字段统一退款，content/subject 双路径反查 owner。

### 微服务集成（RabbitMQ dispatch + HTTP callback）
- 7 微服务（knowledge_video / code_video / interactive_html / knowledge_explanation / pretest / plan / quiz）独立 direct exchange `zhiying.{service}`，绑定 queue `zhiying.{service}.generate`，routing key `generate`。
- 启动时 `declare_topology` 幂等声明，durable + `delivery_mode=2` + publisher confirm。nack/连接失败 → `BusinessError::ServiceUnavailable`，与同步路径一致退款。
- 抽象 `MessagePublisher` trait：生产用 `LapinPublisher`（`deadpool_lapin` 池），测试用 `InMemoryPublisher`（支持 `fail_next`）。
- callback：内容类用 `PATCH /internal/{resource}/{id}`，学习主题类用 `POST /internal/{resource}/{id}`。

### 测试
- 集成测试覆盖：认证、签到、学习主题完整链路、四类资源 dispatch + 回调 + 退款、quiz dispatch payload、管理员充值、wiremock + InMemoryPublisher 双轨。

## 进行中 / 未完成

- **搜索**：所有 list 接口尚未支持 `q` 参数。计划加在 study-subject / knowledge-video / code-video / interactive-html / mistake-list。
- **用户中心**：改密、删号、消费明细、活跃 session 列表均未做。
- **资源分享**：`public` 字段已存在但无切换入口、无公开浏览端点。
- **钻石商店 / 充值入口**：仅有内部充值接口，缺面向用户的购买流程。
- **任务派生资源加入工具画廊**：双路径目前完全隔离，未做反向 link。
- **mistake 详情聚合接口**：当前仅列表，详情走 quiz_problem 路径。
- **占位路由 `placeholders::router()`**：见 `src/routes/placeholders.rs`，待替换。
- **profile 测试期望**：`profile_get_returns_default_values` 仍按旧注册奖励 0 钻石断言，需校准为 `REGISTER_BONUS_DIAMONDS`（当前 80）。

## 临时决策

- migration 单文件 + 删库重建，等需要兼容旧库再切到追加。
- 任务派生与工具自由创建共用 resource 表，但 ownership 完全分离，避免后续画廊混杂。
- callback FAILED 退款 owner 反查走 link 优先，task 次之；都查不到时跳过退款 + warn 日志（理论上不可能）。

## 下一步建议

1. 给 list 接口加 `q` 参数，前端 Dashboard 搜索栏点亮的前置依赖。
2. `placeholders` 中的占位路由按需替换为正式实现。
3. 校准 `profile_get_returns_default_values` 测试。
