# WinNotify — Windows 通知管理器

基于 **C++/WinRT + WinUI 3** 的轻量级 Windows 通知管理器。从右下角弹出通知卡片，并提供通知中心统一管理历史通知。

## 功能

- 通知弹窗：从屏幕右下角滑入，圆角 + 半透明 + 阴影，自动消失 / 手动关闭，可堆叠
- 通知中心：历史记录持久化，已读/未读，按分类分组，关键字搜索 + 分类筛选
- 实时推送：WebSocket 或 HTTP 长轮询两种后端接入方式，自动重连
- 点击回调：点击通知可打开链接（`actionUrl`）或触发自定义动作（`actionId`）
- 系统托盘：关闭窗口最小化到托盘，托盘菜单支持打开/暂停接收/退出
- 通知分类（消息/提醒/系统/其他）与优先级（低/普通/高）标记
- 轻量：空闲时无后台轮询开销（WebSocket 事件驱动），低内存低 CPU

## 构建

环境要求：Windows 10 1809+（推荐 Windows 11）、Visual Studio 2022（v143 工具集）、Windows App SDK 1.5+。

```
WinNotify.sln   用 VS2022 打开
```

工程为免打包（unpackaged）+ 自包含（`WindowsAppSDKSelfContained=true`），构建产物为独立可执行文件，无需 MSIX 安装。首次构建会通过 NuGet 还原 `Microsoft.WindowsAppSDK` 与 `Microsoft.Windows.CppWinRT`。

发布配置请选择 `x64 / Release`，产物位于 `bin\x64\Release\`。

## 运行与配置

配置文件 `config.json` 位于可执行文件同目录，首次启动自动生成默认配置。

```json
{
  "backend": {
    "type": "websocket",        // websocket | poll
    "url": "ws://127.0.0.1:9001/ws",
    "pollIntervalSec": 30
  },
  "popup": {
    "enabled": true,            // 是否弹窗
    "durationSec": 6,           // 弹窗显示秒数
    "width": 380,
    "maxStack": 5               // 同屏最大堆叠数
  },
  "sound": { "enabled": false },
  "startMinimized": true,       // 启动即最小化到托盘
  "history": { "maxItems": 500 },
  "filters": [
    { "pattern": "spam", "enabled": true, "block": true }
  ],
  "mutedCategories": []          // 屏蔽的分类，如 ["system"]
}
```

- `filters.block = true`：命中关键词的通知被拦截；`block = false`：白名单，仅放行命中项
- `mutedCategories`：按分类静音
- 历史记录持久化在 `%LOCALAPPDATA%\WinNotify\history.json`

### 后端协议

服务端向客户端推送的 JSON（单条通知或数组）字段：

| 字段 | 说明 |
|------|------|
| id | 通知唯一 ID（长轮询用作 since 增量游标） |
| title / body | 标题 / 正文 |
| app | 来源应用名（可选） |
| category | message / reminder / system / other |
| priority | low / normal / high |
| actionUrl | 点击时打开链接（可选） |
| actionId | 自定义动作标识（可选） |

WebSocket：连接后服务端逐条发送文本帧 JSON。长轮询：`GET {url}?since=<lastId>`，服务端挂起连接，有新通知时立即返回 JSON 数组，否则超时返回空数组。

### 联调演示后端

```
python3 tools/demo_server.py
```

提供两路推送：`ws://127.0.0.1:9001/ws`（WebSocket，每 4 秒推一条）与 `http://127.0.0.1:9002/poll`（HTTP 长轮询，挂起 25 秒）。配合默认配置即可直接体验，长轮询模式请把 `backend.url` 改为 `http://127.0.0.1:9002/poll` 且 `type` 设为 `poll`。

## 目录结构

```
core/              核心逻辑（不依赖 UI 框架层细节）
  Config.*         配置加载/保存、过滤规则、数据模型
  NotifyItem.*     通知数据模型
  NotifyCenter.*   通知仓库（增删、已读、持久化）
  BackendClient.*  后端客户端（WebSocket + 长轮询）
  Toast.*          通知弹窗窗口（动画/阴影/自动消失）
  ToastMgr.*       弹窗堆叠管理
  TrayIcon.*       系统托盘
  Util.*           工具函数
App.xaml.*         应用装配与生命周期
MainWindow.xaml.*  通知中心窗口
config.json        运行时配置模板
```
