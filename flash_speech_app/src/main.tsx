import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
window.onerror = (message, source, lineno, colno, error) => {
    document.body.innerHTML = `<div style="background:red;color:white;padding:20px;z-index:9999;position:fixed;top:0;left:0;right:0;bottom:0;word-break:break-all;">Error: ${message}<br/>${error?.stack}</div>`;
};

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
        <App />
    </React.StrictMode>
);
