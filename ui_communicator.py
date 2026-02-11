import json
import time
import socket
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

DEFAULT_PORT = 56789


class UICommunicator:
    def __init__(self, on_exit=None, port=None):
        self._state = {"state": "idle", "text": "", "timestamp": time.time()}
        self._lock = threading.Lock()
        self._on_exit = on_exit
        self._sse_clients = []
        self._sse_clients_lock = threading.Lock()
        self._server = None
        self.port = port or DEFAULT_PORT
        self._start_server()
        print(f"UI Server started on port {self.port}")

    def _start_server(self):
        communicator = self

        class StateHandler(BaseHTTPRequestHandler):
            def _send_json(self, code, body):
                self.send_response(code)
                self.send_header('Content-Type', 'application/json')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()
                self.wfile.write(json.dumps(body).encode('utf-8'))

            def do_OPTIONS(self):
                self.send_response(200)
                self.send_header('Access-Control-Allow-Origin', '*')
                self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
                self.send_header('Access-Control-Allow-Headers', 'Content-Type')
                self.end_headers()

            def do_GET(self):
                if self.path == '/events':
                    self._handle_sse()
                    return
                # 普通 GET 仍然返回当前状态（兼容轮询）
                with communicator._lock:
                    state_copy = dict(communicator._state)
                self._send_json(200, state_copy)

            def _handle_sse(self):
                """Server-Sent Events 端点，推送状态变化"""
                self.send_response(200)
                self.send_header('Content-Type', 'text/event-stream')
                self.send_header('Cache-Control', 'no-cache')
                self.send_header('Connection', 'keep-alive')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()

                # 先发送当前状态
                with communicator._lock:
                    state_copy = dict(communicator._state)
                data = json.dumps(state_copy)
                try:
                    self.wfile.write(f"data: {data}\n\n".encode('utf-8'))
                    self.wfile.flush()
                except Exception:
                    return

                # 注册为 SSE 客户端
                with communicator._sse_clients_lock:
                    communicator._sse_clients.append(self.wfile)

                # 保持连接打开，直到客户端断开
                try:
                    while True:
                        time.sleep(1)
                        # 发送心跳保持连接
                        self.wfile.write(": heartbeat\n\n".encode('utf-8'))
                        self.wfile.flush()
                except Exception:
                    pass
                finally:
                    with communicator._sse_clients_lock:
                        if self.wfile in communicator._sse_clients:
                            communicator._sse_clients.remove(self.wfile)

            def do_POST(self):
                content_length = int(self.headers.get('Content-Length', 0))
                post_data = self.rfile.read(content_length)
                try:
                    data = json.loads(post_data.decode('utf-8'))
                    if data.get('action') == 'exit':
                        if communicator._on_exit:
                            communicator._on_exit()
                        self._send_json(200, {"status": "exiting"})
                        return
                except Exception as e:
                    print(f"Error handling POST: {e}")

                self._send_json(400, {"status": "error"})

            def log_message(self, format, *args):
                return

        HTTPServer.allow_reuse_address = True
        self._server = HTTPServer(('127.0.0.1', self.port), StateHandler)
        thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        thread.start()

    def _broadcast_sse(self, state_dict):
        """向所有 SSE 客户端推送状态更新"""
        data = json.dumps(state_dict)
        message = f"data: {data}\n\n".encode('utf-8')
        with self._sse_clients_lock:
            dead_clients = []
            for client in self._sse_clients:
                try:
                    client.write(message)
                    client.flush()
                except Exception:
                    dead_clients.append(client)
            for c in dead_clients:
                self._sse_clients.remove(c)

    def update(self, state, text=""):
        with self._lock:
            self._state["state"] = state
            self._state["text"] = text
            self._state["timestamp"] = time.time()
            state_copy = dict(self._state)
        # 在锁外广播，避免死锁
        self._broadcast_sse(state_copy)

    def get_state(self):
        with self._lock:
            return self._state["state"]

    def shutdown(self):
        """关闭 HTTP 服务器"""
        if self._server:
            self._server.shutdown()
            self._server = None
