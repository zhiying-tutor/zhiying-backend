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

项目已经完成基础后端骨架，当前可编译运行，已实现的核心能力包括：

- 用户注册：`POST /api/v1/users`
- 用户登录：`POST /api/v1/tokens`
- 当前用户信息：`GET /api/v1/me`
- 更新当前用户信息：`PATCH /api/v1/me`
- 签到：`POST /api/v1/checkins`
- 查询签到记录：`GET /api/v1/checkins`
- 健康检查：`GET /health`

其余业务模块当前保留了占位路由，后续继续按资源领域补齐。

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

核心配置项包括：

- `DATABASE_URL`
- `JWT_SECRET`
- `JWT_TTL_DAYS`
- `CORS_ALLOW_ORIGIN`
- `CHECKIN_REWARD_SEQUENCE`
- `CHECKIN_MAKEUP_GOLD_COST_PER_DAY`
- `CHECKIN_MAKEUP_DIAMOND_COST`

默认配置中：

- JWT 过期时间为 30 天
- 连续签到奖励序列为 `1,2,3,4,6,8,10`
- 补签金币成本为每断签 1 天消耗 50 金币
- 补签钻石成本为每次补签固定消耗 1 钻石

## 项目结构

```text
src/
  routes/      HTTP 路由与 handler，按资源领域拆分
  entities/    SeaORM entity
  services/    业务逻辑与通用逻辑
  config.rs    环境变量配置
  error.rs     统一错误模型与错误响应
  bootstrap.rs 启动期数据库初始化
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
