import json
import time
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

# Shared state
CURRENT_STATE = {"state": "idle", "text": ""}
ON_EXIT_CALLBACK = None
PORT = 56789

class StateHandler(BaseHTTPRequestHandler):
    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()

    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*') # CORS
        self.end_headers()
        self.wfile.write(json.dumps(CURRENT_STATE).encode('utf-8'))

    def do_POST(self):
        content_length = int(self.headers['Content-Length'])
        post_data = self.rfile.read(content_length)
        try:
            data = json.loads(post_data.decode('utf-8'))
            if data.get('action') == 'exit':
                if ON_EXIT_CALLBACK:
                    ON_EXIT_CALLBACK()
                self.send_response(200)
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()
                self.wfile.write(b'{"status": "exiting"}')
                return
        except Exception as e:
            print(f"Error handling POST: {e}")
        
        self.send_response(400)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(b'{"status": "error"}')

    def log_message(self, format, *args):
        return # Silence logs

def run_server():
    # Allow address reuse to avoid "Address already in use" errors during restart
    HTTPServer.allow_reuse_address = True
    server = HTTPServer(('127.0.0.1', PORT), StateHandler)
    server.serve_forever()

# Start server in background thread immediately upon import? 
# Better to do it in __init__
server_thread = threading.Thread(target=run_server, daemon=True)
server_thread.start()

class UICommunicator:
    def __init__(self, on_exit=None):
        global ON_EXIT_CALLBACK
        if on_exit:
            ON_EXIT_CALLBACK = on_exit
        print(f"UI Server started on port {PORT}")

    def update(self, state, text=""):
        global CURRENT_STATE
        CURRENT_STATE["state"] = state
        CURRENT_STATE["text"] = text
        CURRENT_STATE["timestamp"] = time.time()

    def get_state(self):
        global CURRENT_STATE
        return CURRENT_STATE["state"]
