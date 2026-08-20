# my-notice-app

基于 Rust + [GPUI 2.0](https://crates.io/crates/gpui) 与 [gpui-component](https://crates.io/crates/gpui-component)
的桌面通知盒：通过 SSE 实时接收服务端推送的通知，以卡片列表 + 右上角弹窗动画展示，
支持已读/未读管理、级别配色、筛选、主题切换与清空。

## 特性

- SSE 实时接收：自动重连（指数退避 + 随机抖动）、心跳超时检测、断线状态提示
- 通知卡片列表：级别图标/配色（info/success/warning/error）、标题、正文、来源、相对时间
- 弹窗动画：每条新通知在窗口右上角弹出（带进入/退出动画），按通知 id 去重替换
- 已读管理：点击卡片标记已读，支持全部/未读/已读筛选、一键已读、清空
- 主题切换：浅色 / 深色两态
- 发送测试：内置「发送测试」按钮，通过服务端 `/send` 推送测试通知
- 容错数据模型：时间戳兼容秒/毫秒/RFC3339，`type` 关键字序列化，`level`/`severity` 别名

## 快速开始

### 1. 启动 SSE 测试服务端

```bash
python3 scripts/sse_test_server.py
```

默认监听 `127.0.0.1:8000`。

### 2. 运行应用

```bash
cargo run
```

应用默认连接 `http://127.0.0.1:8000`。可用环境变量覆盖：

```bash
NOTICE_SERVER_URL=http://127.0.0.1:9000 cargo run
```

### 3. 推送一条测试通知

```bash
curl -X POST http://127.0.0.1:8000/send \
  -H 'Content-Type: application/json' \
  -d '{"title": "你好", "message": "第一条通知", "type": "success"}'
```

## SSE 协议

服务端 `GET /events` 返回 `text/event-stream` 流。应用解析 `event:` 与 `data:` 字段
（`data` 为 JSON）。

| 事件名 | data 报文 | 作用 |
|--------|-----------|------|
| `notification` / `notice` / `message` / 无事件名 | 见下方「通知报文」 | 新增/更新通知 |
| `read` | `{"id": "..."}` 或 `{"notification_id": "..."}` | 将指定通知标记已读 |
| `read_all` / `readAll` | `{}` | 全部标记已读 |
| `clear` / `clear_all` / `clearAll` | `{}` | 清空通知列表 |
| `connected` / `ready` | `{"server_time": "..."}` | 连接握手（静默处理） |
| `ping` / `heartbeat` / `keepalive` | `{}` | 心跳（静默处理） |

### 通知报文

```json
{
  "id": "n-1",
  "type": "success",
  "title": "构建完成",
  "message": "v1.2.0 发布",
  "source": "ci",
  "created_at": 1720000000000,
  "link": "https://example.com/build/1",
  "metadata": { "job": "release" }
}
```

- `type`：`info` / `success` / `warning` / `error`，同时兼容 `level`、`severity` 别名，未知值回退 `info`
- `created_at`：毫秒/秒时间戳或 RFC3339 字符串，缺省为当前时间
- `source` / `link` / `metadata` / `read`：可缺省

## 构建发布

GitHub Actions 工作流位于 `.github/workflows/build.yml`，在 ubuntu / windows / macos
三平台构建 release 二进制，产物重命名为 `my-notice-app-{linux|windows|macos}-x86_64`。
推送形如 `v*` 的 tag 时自动创建 GitHub Release 并附带产物。

## 依赖与平台说明

- Rust 2024 edition，依赖 GPUI 0.2.2 与 gpui-component 0.5.1（crates.io 稳定版）
- Linux 构建需要若干 X11/Wayland 系统库，例如 `libxcb-cursor-dev`、`libxkbcommon-x11-dev` 等

## 测试

```bash
cargo test
```

单元测试覆盖 SSE 报文解析、通知模型容错反序列化等。
