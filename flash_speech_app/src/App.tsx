import { useState, useEffect, useRef } from 'react';
import { HUD } from './components/HUD';
import './index.css';

type AppState = 'idle' | 'listening' | 'processing' | 'result' | 'disconnected' | 'exiting' | 'error';

function App() {
    const [state, setState] = useState<AppState>('disconnected');
    const [text, setText] = useState('');
    const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

    useEffect(() => {
        let eventSource: EventSource | null = null;
        let cancelled = false;

        function connect() {
            if (cancelled) return;

            eventSource = new EventSource('http://127.0.0.1:56789/events');

            eventSource.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    setState(data.state);
                    if (data.text) setText(data.text);
                } catch (e) {
                    console.error('SSE parse error:', e);
                }
            };

            eventSource.onopen = () => {
                // 连接成功，清除重连计时器
                if (reconnectTimer.current) {
                    clearTimeout(reconnectTimer.current);
                    reconnectTimer.current = null;
                }
            };

            eventSource.onerror = () => {
                setState('disconnected');
                eventSource?.close();
                eventSource = null;
                // 断线重连，2 秒后重试
                if (!cancelled) {
                    reconnectTimer.current = setTimeout(connect, 2000);
                }
            };
        }

        connect();

        return () => {
            cancelled = true;
            eventSource?.close();
            if (reconnectTimer.current) {
                clearTimeout(reconnectTimer.current);
            }
        };
    }, []);

    return (
        <div className="w-full h-full flex items-center justify-center bg-transparent">
            <HUD state={state} text={text} />
        </div>
    );
}

export default App;
