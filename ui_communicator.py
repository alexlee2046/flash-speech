import json
import time
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

# Shared state
CURRENT_STATE = {"state": "idle", "text": ""}
PORT = 56789

class StateHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*') # CORS
        self.end_headers()
        self.wfile.write(json.dumps(CURRENT_STATE).encode('utf-8'))

    def log_message(self, format, *args):
        return # Silence logs

def run_server():
    server = HTTPServer(('127.0.0.1', PORT), StateHandler)
    server.serve_forever()

# Start server in background thread immediately upon import? 
# Better to do it in __init__
server_thread = threading.Thread(target=run_server, daemon=True)
server_thread.start()

class UICommunicator:
    def __init__(self):
        print(f"UI Server started on port {PORT}")

    def update(self, state, text=""):
        global CURRENT_STATE
        CURRENT_STATE["state"] = state
        CURRENT_STATE["text"] = text
        CURRENT_STATE["timestamp"] = time.time()
