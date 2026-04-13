# zhiying-backend

`zhiying-tutor` 的后端服务，使用 Rust 构建，当前采用 `axum + tokio + sea-orm`。

## 技术栈

- `axum`
- `tokio`
- `sea-orm`
- `argon2`
- `jsonwebtoken`

数据库连接当前支持：

- `sqlite`
- `postgresql`
- `mysql`

## 当前状态

项目已完成用户/认证/签到核心链路、4 种独立内容生成资源的异步任务架构，以及完整的学习主题模块（课前测、学习计划、阶段/任务顺序解锁、小测、题目管理）。

## API 概览

### 用户与认证

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/api/v1/users` | 用户注册 |
| `POST` | `/api/v1/tokens` | 用户登录 |
| `GET` | `/api/v1/me` | 当前用户信息 |
| `PATCH` | `/api/v1/me` | 更新当前用户信息 |

### 签到

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/api/v1/checkins` | 签到（支持补签） |
| `GET` | `/api/v1/checkins` | 查询签到记录 |

### 内容生成

4 种独立资源：`knowledge-videos`、`code-videos`、`interactive-htmls`、`knowledge-explanations`，每种资源接口格式一致：

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/api/v1/{resource}` | 创建生成任务（扣费） |
| `GET` | `/api/v1/{resource}/{id}` | 查询任务状态 |
| `PATCH` | `/api/v1/{resource}/{id}` | 设置公开 / 重新生成 |

### 学习主题

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/api/v1/study-subjects` | 创建学习主题（扣钻石，触发课前测生成） |
| `GET` | `/api/v1/study-subjects` | 列表 |
| `GET` | `/api/v1/study-subjects/{id}` | 详情 |
| `GET` | `/api/v1/study-subjects/{id}/pretest` | 获取课前测题目 |
| `PATCH` | `/api/v1/study-subjects/{id}/pretest/{pretest_problem_id}` | 更新课前测答案 |
| `POST` | `/api/v1/study-subjects/{id}/plan` | 提交课前测并创建学习计划 |

### 学习阶段与任务

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/v1/study-stages/{id}` | 获取阶段详情（含任务列表） |
| `GET` | `/api/v1/study-tasks/{id}` | 获取任务详情 |
| `POST` | `/api/v1/study-tasks/{id}/complete` | 标记任务完成（触发顺序解锁） |
| `POST` | `/api/v1/study-tasks/{id}/knowledge-video` | 请求生成知识视频（付费） |
| `POST` | `/api/v1/study-tasks/{id}/interactive-html` | 请求生成互动课件（付费） |
| `POST` | `/api/v1/study-tasks/{id}/explanation` | 请求生成知识讲解（免费） |
| `POST` | `/api/v1/study-tasks/{id}/quizzes` | 创建小测 |
| `GET` | `/api/v1/study-tasks/{id}/quizzes` | 获取任务的小测列表 |

### 小测

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/v1/study-quizzes/{id}` | 获取小测详情（含题目） |
| `PATCH` | `/api/v1/study-quizzes/{quiz_id}/problems/{study_quiz_problem_id}` | 更新小测题目答案 |
| `POST` | `/api/v1/study-quizzes/{id}/submit` | 提交小测 |

### 题目管理

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/v1/problems` | 题目列表（支持 `?bookmarked=true&wrong=true`） |
| `PATCH` | `/api/v1/problems/{id}/bookmark` | 切换收藏 |

### 内部回调

| 方法 | 路径 | 说明 |
|------|------|------|
| `PATCH` | `/internal/{content-resource}/{id}` | 内容生成微服务回调 |
| `POST` | `/internal/study-subjects/{id}` | 学习主题回调（pretest / plan） |
| `POST` | `/internal/study-quizzes/{id}` | 小测回调 |
| `POST` | `/internal/users/{id}/balance` | 充值（增减金币/钻石，API_KEY 鉴权） |

### 其他

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/health` | 健康检查 |

## 本地启动

1. 准备环境变量：

```bash
cp .env.example .env
```

2. 启动服务：

```bash
cargo run
```

默认监听地址：

- `0.0.0.0:3000`

## 环境变量

可参考 [.env.example](.env.example)。

## 项目结构

```text
src/
  routes/      HTTP 路由与 handler，按资源领域拆分
  entities/    SeaORM entity
  services/    业务逻辑与微服务 dispatch
  migration/   数据库迁移
  config.rs    环境变量配置
  error.rs     统一错误模型与错误响应
  auth.rs      JWT 鉴权与微服务 API Key 鉴权
  main.rs      应用启动入口
```

## 文档

- [AGENTS.md](AGENTS.md)：协作约定与长期规则
- [PROGRESS.md](PROGRESS.md)：当前实现进度与后续计划

## 开发说明

- 新增依赖优先使用 `cargo add`
- 修改后至少执行：

```bash
cargo fmt
cargo check
```

- 新接口优先按资源领域建模
- 面向用户的错误信息默认使用中文
