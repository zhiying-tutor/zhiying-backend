# PROGRESS.md

最后更新：2026-04-01

## 当前状态

项目已从空白 Rust 工程搭成可编译的后端骨架，并已补上首批 SeaORM migration。

当前数据库初始化已改为应用启动时直接执行 migration，不再保留单独的启动期建表封装；当前可通过 `cargo check`。

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
- 为未实现模块保留了占位路由，统一返回未实现错误。
- 已接入 SeaORM migration，启动时会自动执行迁移，初始化当前已落地的核心表：
  - `user`
  - `user_checkin`
- 当前 migration 采用早期开发阶段的顺序编号风格，首个 migration 为 `m0001_init_schema.rs`。
- 已移除无实际独立价值的 `bootstrap.rs` 过渡层，启动入口直接执行 migration。

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

当前默认值中：

- JWT 过期时间为 30 天
- 连续签到奖励序列为 `1,2,3,4,6,8,10`
- 补签金币成本为每断签 1 天消耗 50 金币
- 补签钻石成本为每次补签固定消耗 1 钻石

## 尚未完成

- 学习计划相关实体与接口
- 学习阶段相关实体与接口
- 学习任务相关实体与接口
- 题目与前测相关实体与接口
- 内容生成与公共内容相关实体与接口
- 更多业务表结构与后续 schema 演进
- 更系统的测试

## 下一步建议

1. 为认证与签到补基础集成测试，先锁住现有最小闭环。
2. 按资源领域优先落地 `study_plans`，再逐步展开 `study_stages / study_tasks / problems`。
3. 在新增业务资源时同步补对应 migration；在仍可删库重建的阶段内，优先维护初始化 migration 的清晰度，不急于堆积历史迁移。
