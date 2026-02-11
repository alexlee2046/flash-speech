import { useState, useEffect } from 'react';
import { HUD } from './components/HUD';
import './index.css'; // Will be created later

function App() {
    // Mock state for now, will connect to backend via FS or Event later
    const [state, setState] = useState<'idle' | 'listening' | 'processing' | 'result'>('idle');
    const [text, setText] = useState('');

    useEffect(() => {
        const interval = setInterval(async () => {
            try {
                const res = await fetch('http://127.0.0.1:56789');
                if (res.ok) {
                    const data = await res.json();
                    setState(data.state);
                    if (data.text) setText(data.text);
                }
            } catch (e) {
                // Ignore connection errors (server might not be up)
            }
        }, 100); // 100ms polling

        return () => clearInterval(interval);
    }, []);

    return (
        // Transparent container matching Tauri config
        <div className="w-full h-full flex items-center justify-center bg-transparent">
            <HUD state={state} text={text} />
        </div>
    );
}

export default App;
