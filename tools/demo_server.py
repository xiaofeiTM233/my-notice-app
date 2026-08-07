#!/usr/bin/env python3
"""Demo notification backend for WinNotify.

Serves both transports on one port:
  - ws://127.0.0.1:9001/ws        (WebSocket, push)
  - http://127.0.0.1:9001/poll?since=0   (HTTP long polling)

Run: python3 tools/demo_server.py
"""
import base64
import hashlib
import json
import socket
import struct
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HOST = "127.0.0.1"
WS_PORT = 9001
HTTP_PORT = 9002
GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

STORE = []
LOCK = threading.Lock()
SEED = 0


def make_notif():
    global SEED
    with LOCK:
        SEED += 1
        i = SEED
    cats = ["message", "reminder", "system", "other"]
    pris = ["low", "normal", "high"]
    n = {
        "id": str(1000 + i),
        "title": "演示通知 #%d" % i,
        "body": "这是第 %d 条由演示后端推送的通知，点击可打开链接。" % i,
        "app": "demo-backend",
        "category": cats[i % len(cats)],
        "priority": pris[i % len(pris)],
        "actionUrl": "https://www.bing.com?q=notify%d" % i,
        "actionId": "demo:%d" % i,
    }
    with LOCK:
        STORE.append(n)
    return n


def ws_accept(key):
    return base64.b64encode(
        hashlib.sha1((key + GUID).encode()).digest()).decode()


def ws_encode(payload):
    b = payload.encode()
    header = bytearray()
    header.append(0x81)
    if len(b) < 126:
        header.append(len(b))
    elif len(b) < 65536:
        header.append(126)
        header += struct.pack(">H", len(b))
    else:
        header.append(127)
        header += struct.pack(">Q", len(b))
    return bytes(header) + b


def serve_ws(conn):
    buf = b""
    while b"\r\n\r\n" not in buf:
        chunk = conn.recv(4096)
        if not chunk:
            return
        buf += chunk
    head = buf.split(b"\r\n\r\n", 1)[0].decode(errors="ignore")
    headers = {}
    for line in head.split("\r\n")[1:]:
        if ":" in line:
            k, v = line.split(":", 1)
            headers[k.strip().lower()] = v.strip()
    if headers.get("upgrade", "").lower() != "websocket":
        conn.sendall(b"HTTP/1.1 426 Upgrade Required\r\n\r\n")
        return
    key = headers.get("sec-websocket-key", "")
    resp = (
        "HTTP/1.1 101 Switching Protocols\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        "Sec-WebSocket-Accept: %s\r\n\r\n" % ws_accept(key)
    )
    conn.sendall(resp.encode())
    try:
        while True:
            conn.sendall(ws_encode(json.dumps(make_notif(), ensure_ascii=False)))
            time.sleep(4)
    except OSError:
        pass
    finally:
        conn.close()


def ws_listener():
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind((HOST, WS_PORT))
    srv.listen(8)
    print("WebSocket listening on ws://%s:%d/ws" % (HOST, WS_PORT))
    while True:
        conn, _ = srv.accept()
        threading.Thread(target=serve_ws, args=(conn,), daemon=True).start()


class PollHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        q = urllib.parse.urlparse(self.path)
        if q.path not in ("/", "/poll"):
            self.send_error(404)
            return
        since = int(urllib.parse.parse_qs(q.query).get("since", ["0"])[0])
        deadline = time.time() + 25
        while time.time() < deadline:
            with LOCK:
                items = [n for n in STORE if int(n["id"]) > since]
            if items:
                break
            time.sleep(0.3)
        body = json.dumps(items, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


def main():
    threading.Thread(target=ws_listener, daemon=True).start()
    srv = ThreadingHTTPServer((HOST, HTTP_PORT), PollHandler)
    print("Long-poll listening on http://%s:%d/poll" % (HOST, HTTP_PORT))
    srv.serve_forever()


if __name__ == "__main__":
    main()
