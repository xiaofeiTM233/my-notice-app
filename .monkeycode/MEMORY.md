# User Instruction Memory

This file records user instructions, preferences, and teachings for reference in future interactions.

## Format

### User Instruction Entry
User instruction entries should follow this format:

[User Instruction Summary]
- Date: [YYYY-MM-DD]
- Context: [Mentioned scenario or time]
- Instructions:
  - [Content of user teaching or instruction, described line by line]

### Project Knowledge Entry
Entries discovered by the Agent during task execution should follow this format:

[Project Knowledge Summary]
- Date: [YYYY-MM-DD]
- Context: Discovered by Agent while performing [specific task description]
- Category: [Operations & Deployment|Build Methods|Testing Methods|Troubleshooting & Debugging|Workflow & Collaboration|Environment Configuration]
- Instructions:
  - [Specific knowledge points, described line by line]

## Deduplication Strategy
- Before adding a new entry, check for similar or identical instructions.
- If a duplicate is found, skip the new entry or merge it with the existing one.
- When merging, update the context or date information.
- This helps avoid redundant entries and keeps the memory file tidy.

## Entries

[Project Knowledge Summary]
- Date: 2026-08-20
- Context: Discovered by Agent while building my-notice-app (Rust + GPUI 桌面应用) 编译与测试
- Category: Build Methods / Environment Configuration
- Instructions:
  - Rust 构建、`cargo test`、`cargo clippy` 前必须执行 `. "$HOME/.cargo/env"` 加载 rustup 环境
  - 本环境总内存约 8GB，编译/链接峰值内存高，构建与测试任务须用 background_terminal_create 并设置 memory_percent/cpu_percent 限额（经验值 memory_percent≈45，cpu_percent≈200），严禁直接前台运行
  - `cargo test` 包含 doctest，链接极慢（约 15 分钟以上），输出经 grep 管道过滤时会有缓冲延迟，需耐心轮询日志文件
  - Linux 下链接失败报 libxkbcommon 相关 cc 错误时，需安装系统库：apt-get install -y libxkbcommon-x11-dev
  - 环境无显示器（DISPLAY 为空），GUI 冒烟测试需 xvfb-run 或跳过 GUI 验证
  - SSE 测试服务端：`python3 scripts/sse_test_server.py`（默认 127.0.0.1:8000），用 curl 可独立验证 `/events` 与 `/send`
