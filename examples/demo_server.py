"""
演示通知后端（HTTP 长轮询模式）

启动:  python3 examples/demo_server.py [port]
配合 config.toml:
  [backend]
  mode = "poll"
  url  = "http://127.0.0.1:9000/notifications"

推送一条通知:
  curl -X POST http://127.0.0.1:9000/send -d '{
    "title": "构建完成",
    "body": "wnotify v0.1.0 发布成功",
    "category": "system",
    "priority": "normal",
    "url": "https://example.com"
  }'
"""
import json, sys, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

lock = threading.Lock()
feed = []  # [{...,"ts": unix}]


def seed():
    now = int(time.time())
    with lock:
        feed.extend([
            {"id": "seed-1", "title": "欢迎使用 wnotify", "body": "这是演示后端推送的第一条通知",
             "category": "system", "priority": "normal", "ts": now},
            {"id": "seed-2", "title": "会议提醒", "body": "15:00 项目周会（302 会议室）",
             "category": "reminder", "priority": "high", "url": "https://example.com/meeting", "ts": now},
            {"id": "seed-3", "title": "张三", "body": "今晚一起吃饭吗？",
             "category": "message", "priority": "normal", "ts": now},
        ])


class H(BaseHTTPRequestHandler):
    def _json(self, obj, code=200):
        b = json.dumps(obj, ensure_ascii=False).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        u = urlparse(self.path)
        if u.path == "/notifications":
            since = int(parse_qs(u.query).get("since", ["0"])[0])
            with lock:
                items = [m for m in feed if m["ts"] > since]
            self._json(items)
        else:
            self._json({"error": "not found"}, 404)

    def do_POST(self):
        if urlparse(self.path).path == "/send":
            n = int(self.headers.get("Content-Length", 0))
            try:
                msg = json.loads(self.rfile.read(n) or b"{}")
            except Exception:
                return self._json({"error": "bad json"}, 400)
            msg.setdefault("id", f"m-{int(time.time()*1000)}")
            msg.setdefault("ts", int(time.time()))
            msg.setdefault("category", "message")
            msg.setdefault("priority", "normal")
            with lock:
                feed.append(msg)
            self._json({"ok": True, "id": msg["id"]})
        else:
            self._json({"error": "not found"}, 404)

    def log_message(self, *a):
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 9000
    seed()
    print(f"demo backend on http://127.0.0.1:{port}/notifications")
    ThreadingHTTPServer(("127.0.0.1", port), H).serve_forever()
