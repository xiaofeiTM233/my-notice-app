#!/usr/bin/env python3
"""my-notice-app 的 SSE 测试服务端。

用法:
    python3 scripts/sse_test_server.py [--host HOST] [--port PORT]

端点:
    GET  /events   SSE 流（通知推送入口）
    POST /send     推送一条通知（JSON body: title/message/type/source...）

手动测试:
    curl -N http://127.0.0.1:8000/events
    curl -X POST http://127.0.0.1:8000/send \\
        -H 'Content-Type: application/json' \\
        -d '{"title": "你好", "message": "第一条通知", "type": "success"}'
"""

import argparse
import json
import queue
import threading
import time
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

#: 所有已连接的 SSE 客户端各自的发送队列。
_SUBSCRIBERS: set[queue.Queue] = set()
_SUBSCRIBERS_LOCK = threading.Lock()


def broadcast(data: str) -> int:
    """把一条 SSE 报文广播给所有已连接客户端，返回投递数。"""
    with _SUBSCRIBERS_LOCK:
        targets = list(_SUBSCRIBERS)
    delivered = 0
    for sub in targets:
        try:
            sub.put_nowait(data)
            delivered += 1
        except queue.Full:
            pass
    return delivered


def build_notification_payload(body: dict) -> dict:
    """把 POST /send 的请求体整理成标准的通知报文。"""
    level = str(body.get("type") or body.get("level") or "info").lower()
    title = str(body.get("title") or "通知")
    message = str(body.get("message") or "")
    payload = {
        "id": str(body.get("id") or str(uuid.uuid4())),
        "type": level,
        "title": title,
        "message": message,
        "source": str(body.get("source") or "sse-test-server"),
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime()),
    }
    for key in ("link", "read", "metadata"):
        if key in body:
            payload[key] = body[key]
    return payload


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):  # 精简日志
        print("[%s] %s" % (self.address_string(), fmt % args))

    def _send_headers(self, status: int, headers: dict[str, str]) -> None:
        self.send_response(status)
        for key, value in headers.items():
            self.send_header(key, value)
        self.end_headers()

    def do_GET(self):
        if self.path.rstrip("/") != "/events":
            self._send_headers(404, {"Content-Type": "text/plain"})
            self.wfile.write(b"not found")
            return

        self._send_headers(200, {
            "Content-Type": "text/event-stream",
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",
        })

        sub: queue.Queue = queue.Queue(maxsize=128)
        with _SUBSCRIBERS_LOCK:
            _SUBSCRIBERS.add(sub)
        try:
            # 握手事件
            self.wfile.write(
                b"event: connected\n"
                b"data: {\"server_time\": \"%s\"}\n\n"
                % time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime()).encode()
            )
            self.wfile.flush()

            last_beat = time.monotonic()
            while True:
                try:
                    data = sub.get(timeout=5.0)
                    self.wfile.write(("data: %s\n\n" % data).encode("utf-8"))
                    self.wfile.flush()
                    last_beat = time.monotonic()
                except queue.Empty:
                    # 心跳：避免代理/中间层掐断空闲连接
                    if time.monotonic() - last_beat >= 15:
                        self.wfile.write(b": heartbeat\n\n")
                        self.wfile.flush()
                        last_beat = time.monotonic()
        except (BrokenPipeError, ConnectionResetError):
            pass
        finally:
            with _SUBSCRIBERS_LOCK:
                _SUBSCRIBERS.discard(sub)

    def do_POST(self):
        if self.path.rstrip("/") != "/send":
            self._send_headers(404, {"Content-Type": "text/plain"})
            self.wfile.write(b"not found")
            return

        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length) if length else b"{}"
        try:
            body = json.loads(raw or b"{}")
        except json.JSONDecodeError:
            self._send_headers(400, {"Content-Type": "application/json"})
            self.wfile.write(json.dumps({"ok": False, "error": "invalid json"}).encode())
            return

        payload = build_notification_payload(body)
        delivered = broadcast(json.dumps(payload, ensure_ascii=False))
        self._send_headers(200, {"Content-Type": "application/json"})
        self.wfile.write(json.dumps({"ok": True, "delivered": delivered}, ensure_ascii=False).encode())


def main() -> None:
    parser = argparse.ArgumentParser(description="my-notice-app SSE 测试服务端")
    parser.add_argument("--host", default="127.0.0.1", help="监听地址 (默认 127.0.0.1)")
    parser.add_argument("--port", type=int, default=8000, help="监听端口 (默认 8000)")
    args = parser.parse_args()

    server = ThreadingHTTPServer((args.host, args.port), Handler)
    print("SSE 测试服务端已启动: http://%s:%d" % (args.host, args.port))
    print("  SSE  端点: GET  /events")
    print("  发送端点: POST /send")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n已停止")


if __name__ == "__main__":
    main()
