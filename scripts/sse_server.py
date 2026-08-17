#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""my-notice-app SSE 测试服务端（纯标准库实现）

本地模拟通知服务端：通过 SSE (text/event-stream) 向客户端持续推送通知消息。

用法:
    python scripts/sse_server.py                     # 默认 127.0.0.1:8866，每 6s 推一条
    python scripts/sse_server.py --port 9000         # 修改端口
    python scripts/sse_server.py --interval 2        # 每 2s 推一条
    python scripts/sse_server.py --heartbeat 10      # 每 10s 发送 : ping 心跳
    python scripts/sse_server.py --burst 3           # 启动时立即连发 3 条

客户端连接地址: http://127.0.0.1:8866/events

协议约定（与客户端 notice-sse 模块一致）:
    * GET /events  -> text/event-stream 长连接
    * 每条消息: id/event/data 字段 + 空行定界
    * event: notice, data 为 test.json 结构（NoticeMessage）的 JSON
    * 心跳: ": ping" 注释行
    * 支持 Last-Event-ID 续传（重连时补发该 ID 之后最近的消息）
"""

import argparse
import json
import sys
import threading
import time
from collections import deque
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

# ---------------------------------------------------------------------------
# 演示数据：字段结构与 test.json 完全一致（meta.type / status / priority 等）
# ---------------------------------------------------------------------------
DEMO_NOTICES = [
    {
        "id": "demo-0001",
        "meta": {
            "type": "system",
            "status": "unread",
            "priority": "normal",
            "channel": "ch-system",
            "timestamp": 0,
            "expirein": None,
        },
        "content": {
            "title": "系统升级通知",
            "summary": "通知服务将于本周六 02:00-04:00 升级维护，期间可能短暂中断。",
            "body": "通知服务将于**本周六 02:00-04:00** 进行升级维护，期间推送可能出现短暂延迟。\n\n感谢您的理解与支持。",
            "author": {"name": "系统管理员", "avatar": "att-1001"},
            "cover": None,
            "tags": ["维护", "公告"],
            "entities": [],
        },
        "interaction": {"views": []},
        "extra": {},
    },
    {
        "id": "demo-0002",
        "meta": {
            "type": "interaction",
            "status": "unread",
            "priority": "normal",
            "channel": "ch-social",
            "timestamp": 0,
            "expirein": None,
        },
        "content": {
            "title": "张三 赞了你的评论",
            "summary": "“这个方案看起来很靠谱，我投一票！”",
            "body": "张三 赞了你的评论：\n\n> 这个方案看起来很靠谱，我投一票！",
            "author": {"name": "张三", "avatar": "att-2002"},
            "cover": None,
            "tags": ["互动"],
            "entities": [],
        },
        "interaction": {
            "views": [{"id": "u-zhangsan", "timestamp": 0, "body": "张三 于浏览时点赞"}]
        },
        "extra": {},
    },
    {
        "id": "demo-0003",
        "meta": {
            "type": "transaction",
            "status": "unread",
            "priority": "high",
            "channel": "ch-pay",
            "timestamp": 0,
            "expirein": None,
        },
        "content": {
            "title": "交易成功提醒",
            "summary": "您有一笔 ¥1,280.00 的订单已完成支付。",
            "body": "订单号 `T20260817001` 已完成支付，金额 **¥1,280.00**，预计 1-2 个工作日发货。",
            "author": {"name": "支付中心", "avatar": "att-3001"},
            "cover": None,
            "tags": ["交易", "支付"],
            "entities": ["att-3002"],
        },
        "interaction": {"views": []},
        "extra": {"orderId": "T20260817001"},
    },
    {
        "id": "demo-0004",
        "meta": {
            "type": "security",
            "status": "unread",
            "priority": "urgent",
            "channel": "ch-account",
            "timestamp": 0,
            "expirein": None,
        },
        "content": {
            "title": "异地登录提醒",
            "summary": "您的账号于 20:15 在 上海 通过新设备登录。",
            "body": "检测到您的账号于 **20:15** 在 **上海** 使用新设备登录。\n\n如非本人操作，请立即修改密码并开启二次验证。",
            "author": {"name": "安全中心", "avatar": "att-4001"},
            "cover": None,
            "tags": ["账号安全", "登录"],
            "entities": [],
        },
        "interaction": {
            "views": [{"id": "u-sec", "timestamp": 0, "body": "安全中心已记录本次登录事件"}]
        },
        "extra": {"riskLevel": "high"},
    },
    {
        "id": "demo-0005",
        "meta": {
            "type": "activity",
            "status": "unread",
            "priority": "low",
            "channel": "ch-marketing",
            "timestamp": 0,
            "expirein": None,
        },
        "content": {
            "title": "夏日大促 · 限时 3 天",
            "summary": "全场 8 折起，会员再享 95 折，快去看看吧～",
            "body": "**夏日大促**来啦！全场 **8 折**起，会员再享 **95 折**，活动仅限 3 天。",
            "author": {"name": "活动运营", "avatar": "att-5001"},
            "cover": None,
            "tags": ["促销", "会员"],
            "entities": [],
        },
        "interaction": {"views": []},
        "extra": {},
    },
    {
        "id": "demo-0006",
        "meta": {
            "type": "other",
            "status": "unread",
            "priority": "low",
            "channel": "ch-misc",
            "timestamp": 0,
            "expirein": None,
        },
        "content": {
            "title": "本周数据简报已生成",
            "summary": "你的周报数据统计已就绪，可前往工作台查看。",
            "body": "本周（08-11 ~ 08-17）数据简报已生成，共包含 3 张图表、12 条明细。",
            "author": {"name": "数据助手", "avatar": "att-6001"},
            "cover": None,
            "tags": ["数据", "周报"],
            "entities": [],
        },
        "interaction": {"views": []},
        "extra": {},
    },
]


def build_sse_message(message: dict) -> bytes:
    """把一条通知序列化为 SSE 帧（id/event/data + 空行）。"""
    payload = json.dumps(message, ensure_ascii=False, separators=(",", ":"))
    return (
        f"id: {message['id']}\n"
        f"event: notice\n"
        f"data: {payload}\n\n"
    ).encode("utf-8")


class NoticeSSEServer(ThreadingHTTPServer):
    """持有演示数据与推送状态的服务器对象。"""

    daemon_threads = True

    def __init__(self, addr, handler, interval, heartbeat, burst):
        super().__init__(addr, handler)
        self.interval = interval
        self.heartbeat = heartbeat
        self.burst = burst
        self.lock = threading.Lock()
        self.seq = 0  # 消息序号（用于生成唯一 id）
        self.recent = deque(maxlen=50)  # 最近发送的消息（id -> message），支持 Last-Event-ID
        self.recent_ids = deque(maxlen=50)
        self.last_send_at = time.monotonic()

    def make_notice(self) -> dict:
        """基于演示模板生成一条新通知（新 id + 当前时间戳）。"""
        with self.lock:
            self.seq += 1
            seq = self.seq
        template = DEMO_NOTICES[(seq - 1) % len(DEMO_NOTICES)]
        message = json.loads(json.dumps(template))  # 深拷贝
        now = int(time.time())
        message["id"] = f"{template['id']}-{seq:04d}"
        message["meta"]["timestamp"] = now
        if message["meta"].get("expirein"):
            message["meta"]["expirein"] = now + 24 * 3600
        for view in message.get("interaction", {}).get("views", []):
            view["timestamp"] = now
        return message

    def remember(self, message: dict) -> None:
        with self.lock:
            self.recent.append(message)
            self.recent_ids.append(message["id"])

    def notices_after(self, event_id: str):
        """返回在 event_id 之后发送的消息（用于 Last-Event-ID 续传）。"""
        with self.lock:
            ids = list(self.recent_ids)
            items = list(self.recent)
        try:
            index = ids.index(event_id) + 1
        except ValueError:
            index = 0
        return items[index:]


class SseHandler(BaseHTTPRequestHandler):
    server_version = "NoticeSSE/1.0"
    protocol_version = "HTTP/1.1"

    # ------------------------------------------------------------------
    # GET
    # ------------------------------------------------------------------
    def do_GET(self):  # noqa: N802
        path = self.path.split("?", 1)[0]
        if path in ("/events", "/sse"):
            self._stream_events()
        elif path == "/healthz":
            self._json({"ok": True, "time": int(time.time())})
        else:
            self._json({"error": "not found"}, status=404)

    def _stream_events(self) -> None:
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream; charset=utf-8")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "keep-alive")
        self.send_header("X-Accel-Buffering", "no")
        self.end_headers()

        server = self.server  # type: NoticeSSEServer

        # 支持 Last-Event-ID 续传
        last_event_id = self.headers.get("Last-Event-ID")
        replay = server.notices_after(last_event_id) if last_event_id else []
        for message in replay:
            server.remember(message)
            self._write(build_sse_message(message))
            server.last_send_at = time.monotonic()

        # 启动时连发 burst 条
        for _ in range(max(0, server.burst)):
            message = server.make_notice()
            server.remember(message)
            self._write(build_sse_message(message))
        server.last_send_at = time.monotonic()

        print(f"[sse] 客户端接入: {self.client_address[0]} (replay={len(replay)}, burst={server.burst})")

        try:
            next_heartbeat = time.monotonic() + server.heartbeat
            while True:
                now = time.monotonic()
                if now >= next_heartbeat:
                    self._write(b": ping\n\n")
                    next_heartbeat = now + server.heartbeat
                if server.interval > 0 and (now - server.last_send_at) >= server.interval:
                    message = server.make_notice()
                    server.remember(message)
                    self._write(build_sse_message(message))
                    server.last_send_at = now
                    print(f"[sse] 推送: {message['id']} {message['meta']['type']} {message['content']['title']}")
                time.sleep(0.2)
        except (BrokenPipeError, ConnectionResetError, OSError):
            print(f"[sse] 客户端断开: {self.client_address[0]}")
        except KeyboardInterrupt:
            pass

    def _write(self, data: bytes) -> None:
        self.wfile.write(data)
        self.wfile.flush()

    # ------------------------------------------------------------------
    # POST（供后续扩展：已读回写等）
    # ------------------------------------------------------------------
    def do_POST(self):  # noqa: N802
        path = self.path.split("?", 1)[0]
        if path.startswith("/read/"):
            message_id = path.removeprefix("/read/")
            print(f"[api] 标记已读: {message_id}")
            self._json({"ok": True, "id": message_id})
        elif path == "/clear":
            self._json({"ok": True})
        else:
            self._json({"error": "not found"}, status=404)

    def _json(self, payload: dict, status: int = 200) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):  # 精简默认日志
        pass


def parse_args(argv):
    parser = argparse.ArgumentParser(
        description="my-notice-app SSE 测试服务端（纯标准库）",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--host", default="127.0.0.1", help="监听地址")
    parser.add_argument("--port", type=int, default=8866, help="监听端口")
    parser.add_argument("--interval", type=float, default=6.0, help="自动推送间隔（秒），0 关闭自动推送")
    parser.add_argument("--heartbeat", type=float, default=15.0, help="心跳注释间隔（秒）")
    parser.add_argument("--burst", type=int, default=2, help="客户端接入时立即推送条数")
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    server = NoticeSSEServer(
        (args.host, args.port),
        SseHandler,
        interval=args.interval,
        heartbeat=args.heartbeat,
        burst=args.burst,
    )
    print("=" * 56)
    print("  my-notice-app SSE 测试服务端")
    print(f"  事件流地址: http://{args.host}:{args.port}/events")
    print(f"  健康检查:   http://{args.host}:{args.port}/healthz")
    print(f"  推送间隔:   {args.interval}s | 心跳: {args.heartbeat}s | 接入连发: {args.burst} 条")
    print("  提示: 用 Ctrl+C 退出")
    print("=" * 56)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[server] 已退出")
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
