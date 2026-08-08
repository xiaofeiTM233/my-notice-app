# WinNotify - Windows 通知管理器

基于 C++ + WinUI 3 构建的轻量级 Windows 通知管理器应用。

## 功能特性

### 核心功能
- **桌面通知弹窗** - 从屏幕右下角滑入的动画通知，支持自动消失和手动关闭
- **通知收纳中心** - 统一管理历史通知，支持已读/未读状态、分组显示和搜索筛选
- **实时推送后端** - 支持 WebSocket 和 HTTP 长轮询两种模式接入自定义通知后端
- **通知分类** - 消息、提醒、系统、告警等分类，支持优先级标记（低/普通/高/紧急）

### 界面特性
- 圆角卡片设计，半透明背景，阴影效果
- 深色主题，现代化 WinUI 3 风格
- 系统托盘集成，最小化到托盘后台运行
- 通知点击回调，支持打开链接或触发自定义动作

### 配置管理
- JSON 配置文件，灵活定制后端地址、过滤规则、弹窗参数等
- 通知过滤规则，支持按分类、优先级、关键词屏蔽或静音

## 项目结构

```
WinNotify/
├── CMakeLists.txt          # CMake 构建配置
├── config.json             # 应用配置文件
├── README.md
├── assets/                 # 资源文件
└── src/
    ├── main.cpp            # 应用入口
    ├── pch.h               # 预编译头
    ├── App.xaml/.h/.cpp    # 应用类
    ├── MainWindow.xaml/.h/.cpp  # 主窗口
    ├── models/
    │   ├── Notification.h/.cpp   # 通知数据模型
    │   └── AppConfig.h           # 配置数据模型
    ├── services/
    │   ├── NotificationService.h/.cpp  # 通知核心服务
    │   ├── BackendClient.h/.cpp        # 后端客户端(WS/HTTP)
    │   └── ConfigManager.h/.cpp        # 配置管理器
    └── ui/
        ├── NotificationPopup.xaml/.h/.cpp  # 通知弹窗组件
        ├── NotificationCenter.xaml/.h/.cpp # 通知中心页面
        ├── PopupManager.h/.cpp             # 弹窗管理器
        └── TrayIcon.h/.cpp                 # 系统托盘
```

## 构建要求

- Windows 10 1809+ / Windows 11
- Visual Studio 2022 (带 C++ 桌面开发工作负载)
- Windows App SDK 1.5+
- CMake 3.26+

## 构建步骤

### 使用 Visual Studio
1. 安装 Windows App SDK：https://github.com/microsoft/WindowsAppSDK
2. 打开 Visual Studio Installer，安装 "使用 C++ 的桌面开发" 工作负载
3. 克隆项目后，用 Visual Studio 打开文件夹
4. CMake 自动配置后，选择 WinNotify.exe 启动调试

### 使用命令行
```bash
mkdir build && cd build
cmake .. -G "Visual Studio 17 2022" -A x64
cmake --build . --config Release
```

## 后端协议

### WebSocket 模式
连接后服务端主动推送通知 JSON：
```json
{
  "id": "unique-id",
  "title": "通知标题",
  "content": "通知内容",
  "category": "message",
  "priority": "normal",
  "timestamp": 1700000000,
  "actionUrl": "https://example.com",
  "groupKey": "chat-123",
  "iconPath": ""
}
```

也支持批量推送：
```json
{
  "notifications": [ ... ]
}
```

### HTTP 长轮询模式
GET 请求配置的 `httpUrl`，返回同上的 JSON 格式。

### 认证
通过 `Authorization: Bearer <token>` 请求头传递认证令牌。

## 配置说明

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| backend.mode | 后端模式: websocket / http_longpoll | websocket |
| backend.wsUrl | WebSocket 地址 | ws://localhost:8080/ws |
| backend.httpUrl | HTTP 轮询地址 | http://localhost:8080/api |
| backend.pollIntervalMs | 轮询间隔(ms) | 5000 |
| popup.displayDurationMs | 弹窗显示时长(ms) | 5000 |
| popup.maxVisible | 最大同时显示数 | 3 |
| popup.animationDurationMs | 动画时长(ms) | 300 |
| popup.width | 弹窗宽度(px) | 360 |
| startMinimized | 启动即最小化 | false |
| minimizeToTray | 最小化到托盘 | true |
| closeToTray | 关闭按钮最小化到托盘 | true |
| maxHistory | 最大历史记录数 | 500 |

## 性能设计

- 使用单例模式管理核心服务，减少内存占用
- 通知列表懒加载，历史记录持久化到本地存储
- 弹窗窗口按需创建，避免常驻资源消耗
- 后台线程处理网络连接，UI 线程无阻塞

## License

MIT
