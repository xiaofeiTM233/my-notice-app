# my-notice-app

基于 **Rust + GPUI + gpui-component** 的跨平台桌面通知盒应用：通过 **SSE 协议**连接通知服务端，实时接收、展示与管理通知消息。

![tech](https://img.shields.io/badge/Rust-1.85+-orange) ![platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey) ![license](https://img.shields.io/badge/license-MIT-blue)

## 功能特性

- **SSE 实时接收**：`notice-sse` 模块提供完整 SSE 客户端（心跳超时保护、指数退避重连、`Last-Event-ID` 续传、事件流解析）。
- **通知数据模型**：`notice-model` 严格按照 `test.json`（Apifox 导出）实现 Rust 映射，覆盖 `meta / content / interaction / extra` 全字段与类型、状态、优先级枚举。
- **通知盒 UI**：三栏布局（筛选侧边栏 + 通知卡片列表 + 详情面板），基于 gpui-component 的 `List` / `ListItem` / `Badge` / `Button` / `TitleBar` / `Notification` 组件。
- **消息弹出动画**：新消息到达时列表卡片滑入渐显（GPUI 动画系统），同时窗口右上角弹出 toast 通知。
- **已读 / 未读管理**：点击卡片标记已读、一键全部已读、按未读/已读/类型筛选、未读角标计数、清空已读。
- **本地测试服务端**：`scripts/sse_server.py` 纯标准库实现，一条命令模拟服务端持续推送。
- **CI / 发布**：GitHub Actions 支持 Windows / macOS（x64 + arm64）/ Linux 多平台编译与 Release 发布。

## 项目结构

```text
my-notice-app/
├── Cargo.toml                     # 工作区配置（依赖版本锁定见下文）
├── crates/
│   ├── notice-model/              # 通知数据模型（test.json 映射 + 单测）
│   ├── notice-sse/                # SSE 客户端连接模块（tokio + reqwest + 单测）
│   └── notice-app/                # 桌面主程序（GPUI + gpui-component）
│       └── src/
│           ├── main.rs            # 入口
│           ├── app.rs             # 应用装配（参数解析、初始化、开窗）
│           ├── state.rs           # 通知存储 / 过滤 / 已读状态 / 连接状态
│           └── ui/
│               ├── root.rs        # 主视图（标题栏 + 三栏布局 + SSE 轮询）
│               ├── notice_list.rs # 通知卡片列表（ListDelegate + 滑入动画）
│               ├── detail.rs      # 详情面板（Markdown/HTML 正文渲染）
│               └── mod.rs         # 公共渲染辅助（时间、类型图标与配色）
├── scripts/
│   └── sse_server.py              # Python SSE 测试服务端（纯标准库）
├── .github/workflows/build.yml    # 多平台构建 + GitHub Release
└── README.md
```

## 快速开始

### 1. 启动本地 SSE 测试服务端

需要 Python 3.9+（纯标准库，无需安装依赖）：

```bash
python scripts/sse_server.py
# 事件流地址: http://127.0.0.1:8866/events
# 默认每 6 秒推送一条演示通知；可用 --interval / --heartbeat / --burst 调整
```

### 2. 编译并运行桌面应用

```bash
cargo run -p notice-app
# 或指定其他服务端地址
cargo run -p notice-app -- --server http://127.0.0.1:8866/events
```

服务端地址也可以通过环境变量指定：`NOTICE_SSE_URL=http://127.0.0.1:8866/events cargo run -p notice-app`。

## 依赖说明

全部依赖来自 **crates.io**（无 git 依赖），可离线复现构建：

| 依赖 | 版本 | 说明 |
|---|---|---|
| `gpui` | 0.2.2 | Zed GPUI 官方发布版，内置 Windows / macOS / Linux 平台后端 |
| `gpui-component` | 0.5.1 | 组件库 crates.io 发布版 |
| `gpui-component-assets` | 0.5.1 | 图标资源 crates.io 发布版 |

- `gpui` 0.2.2 将平台后端（Windows / macOS / Linux）合并发布，因此不再需要 `gpui_platform` 及 zed 仓库 git 依赖。
- 三个版本号是配套组合（gpui-component 0.5.1 的依赖即 crates.io 的 gpui 0.2.2），请勿单独升级其中某一个。
- 首次编译需下载数百个依赖（含 wgpu 等），耗时较长，属正常现象；后续构建有缓存会快很多。

## SSE 协议约定（客户端 ↔ 服务端）

请求：

```http
GET /events HTTP/1.1
Accept: text/event-stream
Cache-Control: no-cache
Last-Event-ID: <可选，断线续传>
```

服务端响应（`Content-Type: text/event-stream`）：

```text
id: demo-0001-0001
event: notice
data: {"id":"...","meta":{...},"content":{...},"interaction":{...},"extra":{}}

: ping

```

- `event: notice` + `data` 为 [`test.json`](crates/notice-model/src/lib.rs) 结构的 JSON（`NoticeMessage`）。
- `id:` 用于 `Last-Event-ID` 断线续传。
- `: ping` 注释行作为心跳；客户端 45 秒未收到任何数据会判定连接失效并自动重连。
- 断线后按指数退避重连：1s → 2s → 4s … 上限 30s。

## 通知数据模型（test.json 映射）

| test.json 字段 | Rust 类型 | 说明 |
|---|---|---|
| `id` | `String` | 数据 ID |
| `meta.type` | `NoticeKind` | `system / interaction / transaction / security / activity / other`（未知值兜底 `Unknown`） |
| `meta.status` | `NoticeStatus` | `unread / read / deleted`（未知值兜底 `Unknown`） |
| `meta.priority` | `NoticePriority` | `low / normal / high / urgent`，默认 `low` |
| `meta.channel` | `String` | 频道 ID |
| `meta.timestamp` / `meta.expirein` | `i64` / `Option<i64>` | Unix 秒；`expirein` 为过期时间点，过期消息不再展示与计数 |
| `content.title` | `String` | 标题（必填） |
| `content.summary` / `body` | `Option<String>` | 摘要缺省回退正文；正文支持 Markdown/HTML |
| `content.author` / `cover` / `tags` / `entities` | `Option<…>` / `Vec<String>` | 作者、封面（附件 ID）、标签、附件 |
| `interaction.views` | `Vec<NoticeView>` | 浏览记录 |
| `extra` | `HashMap<String, Value>` | 业务扩展字段（透传展示于详情） |

详细代码见 [`notice-model`](crates/notice-model/src/lib.rs)，含完整单测。

## 本地构建系统依赖

- **Windows**：MSVC 工具链（`stable-x86_64-pc-windows-msvc`）即可，无额外依赖。
- **macOS**：Xcode Command Line Tools。
- **Linux（Ubuntu 24.04 等）**：

  ```bash
  sudo apt install -y gcc g++ clang pkg-config cmake \
      libfontconfig-dev libwayland-dev \
      libxkbcommon-x11-dev libx11-xcb-dev \
      libssl-dev libzstd-dev \
      libvulkan1 vulkan-validationlayers
  ```

## GitHub Actions

`.github/workflows/build.yml` 支持：

- **PR / push main**：Windows、macOS（x64 + arm64）、Linux 四平台构建并上传构建产物；
- **打标签 `v*`**：构建完成后自动创建 GitHub Release，附带各平台二进制。

## 验证说明

- 单元测试共 18 项全部通过：`notice-model`（test.json 结构解析、未知枚举兜底、过期判断等 5 项）、`notice-sse`（SSE 行解析、退避计算、缓冲读取等 5 项）、`notice-app::state`（去重排序、已读管理、过滤、过期隐藏等 8 项）。运行 `cargo test` 即可复现。
- 已在本机（Windows）实测：`cargo build -p notice-app` 成功产出可执行文件，应用启动后窗口正常，并通过 SSE 长连接持续接收测试服务端推送。
- 本地首次编译耗时较长（数百个依赖），属正常现象。

## 后续扩展方向（TODO）

- [ ] 已读状态回写服务端（`POST /read/:id`，测试服务端已预留接口）
- [ ] 通知声音提示与系统托盘
- [ ] 正文 HTML 渲染的完整样式支持
- [ ] 分页加载与本地持久化（历史消息落盘）
- [ ] 多服务端 / 多频道订阅配置
