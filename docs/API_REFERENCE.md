# API Reference

所有路由以 `/api/v2` 为前缀。完整契约以 [`contract/openapi.yaml`](../contract/openapi.yaml) 为单一权威源。

## 认证（公开）

```bash
# 请求验证码
curl -X POST http://localhost:8080/api/v2/auth/email/request-code \
  -H "Content-Type: application/json" \
  -d '{"email":"student@tongji.edu.cn"}'

# 验证码登录
curl -X POST http://localhost:8080/api/v2/auth/email/verify \
  -H "Content-Type: application/json" \
  -d '{"email":"student@tongji.edu.cn","code":"123456"}'

# 刷新令牌
curl -X POST http://localhost:8080/api/v2/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token":"..."}'

# 登出
curl -X POST http://localhost:8080/api/v2/auth/logout \
  -H "Authorization: Bearer <access_token>"
```

## 身份（需认证）

```bash
# 个人信息
curl http://localhost:8080/api/v2/me \
  -H "Authorization: Bearer <access_token>"

# 修改昵称 / 头像
curl -X PATCH http://localhost:8080/api/v2/me \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"handle":"my_new_handle"}'

# 用户公开资料
curl http://localhost:8080/api/v2/users/my_new_handle

# 绑定 Ed25519 公钥
curl -X POST http://localhost:8080/api/v2/wallet/bind \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"publicKey":"<base64_ed25519>"}'

# 查看积分余额
curl http://localhost:8080/api/v2/wallet \
  -H "Authorization: Bearer <access_token>"

# 草稿（自动保存）
curl http://localhost:8080/api/v2/me/drafts \
  -H "Authorization: Bearer <access_token>"

# 忽略用户
curl -X PUT http://localhost:8080/api/v2/me/ignores/2 \
  -H "Authorization: Bearer <access_token>"
```

## 课程 & 选课（公开）

```bash
# 课程列表
curl "http://localhost:8080/api/v2/courses?dept=数学科学学院&sort=hot&limit=20"

# 课程详情
curl http://localhost:8080/api/v2/courses/1

# 搜索
curl "http://localhost:8080/api/v2/search?q=高等数学&type=course&limit=10"

# 选课日历
curl http://localhost:8080/api/v2/selection/calendars
```

## 点评

```bash
# 课程点评列表
curl "http://localhost:8080/api/v2/courses/1/reviews?sort=hot"

# 发布点评（需认证）
curl -X POST http://localhost:8080/api/v2/courses/1/reviews \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"rating":4,"comment":"很好的课程","semester":"2024-2025-1"}'

# 点赞
curl -X POST http://localhost:8080/api/v2/reviews/1/like \
  -H "Authorization: Bearer <access_token>"
```

## 积分（需认证 + X-Wallet-Sig）

```bash
# 打赏
curl -X POST http://localhost:8080/api/v2/credit/tip \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Wallet-Sig: <base64_signature>" \
  -H "Content-Type: application/json" \
  -d '{"toAccountId":"2","amount":10,"targetType":"review","targetId":"1"}'

# 查看账本
curl "http://localhost:8080/api/v2/wallet/ledger" \
  -H "Authorization: Bearer <access_token>"

# 验证账本完整性（公开）
curl http://localhost:8080/api/v2/wallet/ledger/verify
```

## 论坛

```bash
# 板块列表
curl http://localhost:8080/api/v2/forum/boards

# 主题流（hot / new / unread / following）
curl "http://localhost:8080/api/v2/forum/threads?board=1&sort=hot"
curl "http://localhost:8080/api/v2/forum/threads?sort=unread" \
  -H "Authorization: Bearer <access_token>"

# 发帖（需认证）
curl -X POST http://localhost:8080/api/v2/forum/threads \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"boardId":"1","title":"Hello","body":"First post!"}'

# 评论（楼中楼，需认证）
curl -X POST http://localhost:8080/api/v2/forum/threads/1/comments \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"body":"Great post!","parentId":null}'

# 顶/踩
curl -X POST http://localhost:8080/api/v2/forum/posts/1/vote \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"value":"up"}'

# 订阅（watching / tracking / muted）
curl -X PUT http://localhost:8080/api/v2/forum/subscriptions \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"targetType":"thread","targetId":"1","level":"watching"}'

# 举报
curl -X POST http://localhost:8080/api/v2/forum/posts/1/flag \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"reason":"spam"}'

# 收藏
curl -X PUT http://localhost:8080/api/v2/forum/posts/1/bookmark \
  -H "Authorization: Bearer <access_token>"

# 通知
curl http://localhost:8080/api/v2/notifications \
  -H "Authorization: Bearer <access_token>"

# 实时通知（SSE）
curl -N http://localhost:8080/api/v2/notifications/stream \
  -H "Authorization: Bearer <access_token>"
```
