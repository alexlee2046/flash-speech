import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { HUD } from './components/HUD';
import './index.css';

type AppState = 'starting' | 'idle' | 'listening' | 'processing' | 'result' | 'disconnected' | 'exiting' | 'error';

function App() {
    const [state, setState] = useState<AppState>('starting');
    const [text, setText] = useState('');

    useEffect(() => {
        const unlisteners: (() => void)[] = [];

        listen<{ state: string; text?: string }>('state-change', (event) => {
            setState(event.payload.state as AppState);
            if (event.payload.text !== undefined) {
                setText(event.payload.text);
            }
        }).then(fn => unlisteners.push(fn));

        return () => { unlisteners.forEach(fn => fn()); };
    }, []);

    return (
        <div className="w-full h-full flex items-center justify-center bg-transparent">
            <HUD state={state} text={text} />
        </div>
    );
}

export default App;
