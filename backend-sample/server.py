#!/usr/bin/env python3
"""notify-hub 后端样例

同时提供两种接入方式（端口/路径与 src/config.rs 默认值一致）：
  - WebSocket:   ws://127.0.0.1:8787/ws
  - HTTP 长轮询: http://127.0.0.1:8787/notifications?since=<unix秒>

通知 JSON 对齐 src/model.rs 的 Notif/Payload：
  ts 为 Unix 秒；cat ∈ message/reminder/system/alert；prio ∈ low/normal/high/urgent
  act 形如 {"type": "url"|"path", "value": "..."}

运行：  python server.py   （依赖：pip install websockets）
调试：  curl 'http://127.0.0.1:8787/notifications?since=0'
        curl -X POST http://127.0.0.1:8787/publish
"""

import asyncio
import json
import threading
import time
from urllib.parse import urlparse, parse_qs

import websockets

HOST, PORT = "127.0.0.1", 8787
POLL_TIMEOUT = 30   # 长轮询最长阻塞秒数
PUSH_EVERY = 5      # WS 连接建立后自动推送间隔（秒）

TEMPLATES = [
    ("新消息", "你有一条来自样例后端的消息", "样例后端", "message", "normal"),
    ("会议提醒", "10 分钟后开始每日站会", "日历", "reminder", "high"),
    ("系统通知", "服务已重新加载配置", "系统", "system", "low"),
    ("安全告警", "检测到异常登录尝试", "安全中心", "alert", "urgent"),
]

store: list = []
seq = 0
store_lock = threading.Lock()
new_event = threading.Event()


def publish():
    """生成一条通知并广播信号，返回该通知。"""
    global seq
    with store_lock:
        t = TEMPLATES[seq % len(TEMPLATES)]
        seq += 1
        n = {
            "id": f"sample-{seq}",
            "title": t[0],
            "body": t[1],
            "src": t[2],
            "cat": t[3],
            "prio": t[4],
            "ts": int(time.time()),
            "act": {"type": "url", "value": "https://example.com"},
        }
        store.append(n)
    new_event.set()
    return n


async def ws_handler(ws):
    await ws.send(json.dumps(publish(), ensure_ascii=False))
    while True:
        await asyncio.sleep(PUSH_EVERY)
        try:
            await ws.send(json.dumps(publish(), ensure_ascii=False))
        except websockets.ConnectionClosed:
            return


def http_process(path: str, _headers):
    """process_request：拦截 HTTP 请求，非 WS 路径返回普通 HTTP 响应。"""
    parsed = urlparse(path)
    p = parsed.path

    # 放行 WS 握手，交给 websockets 升级
    if p == "/ws":
        return None

    if p in ("/", "/health"):
        return (200, {"Content-Type": "text/plain"}, b"notify-hub sample backend OK")

    # 注意：websockets 服务器在调用 process_request 前会拒绝非 GET 请求，
    # 因此 /publish 与 /notifications 均用 GET。
    if p == "/publish":
        n = publish()
        return (200, {"Content-Type": "application/json"},
                json.dumps(n, ensure_ascii=False).encode("utf-8"))

    if p == "/notifications":
        qs = parse_qs(parsed.query)
        try:
            since = int((qs.get("since") or ["0"])[0])
        except ValueError:
            since = 0
        deadline = time.time() + POLL_TIMEOUT
        while True:
            with store_lock:
                fresh = [n for n in store if n["ts"] > since]
            if fresh:
                return (200, {"Content-Type": "application/json"},
                        json.dumps(fresh, ensure_ascii=False).encode("utf-8"))
            remaining = deadline - time.time()
            if remaining <= 0:
                return (200, {"Content-Type": "application/json"}, b"[]")
            new_event.wait(min(0.5, remaining))

    return (404, {"Content-Type": "text/plain"}, b"not found")


async def main():
    publish()
    publish()
    async with websockets.serve(ws_handler, HOST, PORT, process_request=http_process):
        print(f"notify-hub 样例后端已启动")
        print(f"  WS:    ws://{HOST}:{PORT}/ws")
        print(f"  轮询:  http://{HOST}:{PORT}/notifications?since=<unix秒>")
        await asyncio.Future()


if __name__ == "__main__":
    asyncio.run(main())
