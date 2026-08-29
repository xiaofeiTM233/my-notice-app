# wnotify

基于 Rust + Slint 的 Windows 通知管理器：右下角滑入式弹窗、通知收纳中心、WebSocket / HTTP 长轮询双模式后端接入、系统托盘常驻。

## 功能

- 弹窗推送：屏幕右下角滑入动画（340ms ease-out）、半透明圆角卡片、阴影、优先级色条，支持自动消失与手动关闭，多条通知排队依次显示
- 通知中心：未读/已读状态、分类筛选（消息/提醒/系统）、关键词搜索、单条删除、全部已读、清空
- 后端接入：WebSocket 实时推送或 HTTP 轮询，断线自动重连，Bearer token 鉴权
- 分类与优先级：message / reminder / system 三类，low / normal / high 三级，高优先级红色标记
- 点击回调：打开通知携带的链接（浏览器）并标记已读
- 系统托盘：Slint 原生 `SystemTrayIcon`，左键打开通知中心，右键菜单（打开中心/全部已读/退出），关闭窗口即最小化到托盘
- 过滤规则：分类白名单、最低优先级、屏蔽关键词、只看关键词

## 构建与运行

```bash
# Windows（目标平台，femtovg GPU 渲染 + 原生 Win32 托盘）
cargo build --release
./target/release/wnotify.exe

# Linux 开发环境
cargo build
xvfb-run ./target/debug/wnotify
```

首次运行会在 exe 同目录生成默认 `config.toml`。

## 配置文件

```toml
[backend]
mode = "ws"                              # ws | poll
url = "ws://127.0.0.1:9000/ws"           # 后端地址
token = ""                               # Authorization: Bearer <token>
poll_secs = 5                            # 轮询间隔

[popup]
enabled = true                           # 弹窗开关
secs = 6                                 # 自动消失时长
max_queue = 10                           # 弹窗排队上限

[filter]
categories = []                          # 允许的分类，空=全部
min_priority = "low"                     # low | normal | high
block = []                               # 命中即丢弃的关键词
only = []                                # 仅显示命中的关键词，空=不过滤

[ui]
start_minimized = false                  # 启动时仅驻留托盘
```

数据模型定义见 `src/config.rs`。

## 后端协议

WebSocket 模式：服务端持续下发 JSON 文本帧；HTTP 轮询模式：`GET {url}?since=<ts>` 返回 JSON 数组或 `{"items": [...]}`。

```json
{
  "id": "可选，缺省自动生成",
  "title": "通知标题",
  "body": "通知正文",
  "category": "message",   // message | reminder | system
  "priority": "normal",    // low | normal | high
  "url": "https://...",    // 可选，点击打开
  "ts": 0                  // 可选，unix 秒，缺省取本地时间
}
```

演示后端：

```bash
python3 examples/demo_server.py 9000   # 启动
# config.toml 改为 mode = "poll", url = "http://127.0.0.1:9000/notifications"

# 手动推送
curl -X POST http://127.0.0.1:9000/send -d '{"title":"你好","body":"测试通知","priority":"high"}'
```

## 代码结构

```
src/
├── main.rs      # 组装：回调绑定、事件泵、弹窗时序
├── config.rs    # 配置数据模型与加载
├── model.rs     # 通知模型、JSON 解析、过滤规则、相对时间
├── store.rs     # 通知仓库：去重、搜索、分类筛选、模型同步
├── backend.rs   # WebSocket / HTTP 轮询连接器（独立线程）
├── popup.rs     # 弹窗队列管理、屏幕定位
└── ui/
    ├── common.slint  # 数据结构 + 配色
    ├── popup.slint   # 右下角弹窗
    ├── center.slint  # 通知中心主窗口
    └── tray.slint    # 系统托盘
```

## 设计要点

- 零重量级依赖：无 tokio/reqwest，WebSocket 用 tungstenite 同步客户端，HTTP 用 ureq，TLS 统一 ring
- 事件架构：后端线程 -> mpsc channel -> UI 线程事件泵（100ms Timer），弹窗自动消失由 deadline 机制驱动
- 低资源：空闲态 CPU 约 0.02%（debug 构建，Xvfb 下实测）；release 构建启用 `lto = "thin"` + `strip`
- 线程模型：网络阻塞调用全部在 `backend` 线程，UI 线程零阻塞
