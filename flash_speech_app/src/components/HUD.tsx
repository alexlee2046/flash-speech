import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Mic, Zap, Check, WifiOff, AlertTriangle } from 'lucide-react';
import { ContextMenu } from './ContextMenu';

interface HUDProps {
    state: 'idle' | 'listening' | 'processing' | 'result' | 'disconnected' | 'exiting' | 'error';
    text?: string;
}

export function HUD({ state, text }: HUDProps) {
    const waveform = [8, 16, 10, 20, 12, 24, 14, 18, 10, 14, 8, 6];

    const [menuOpen, setMenuOpen] = useState(false);
    const [menuPos, setMenuPos] = useState({ x: 0, y: 0 });

    const handleContextMenu = (e: React.MouseEvent) => {
        e.preventDefault();
        // 边界检测：确保菜单不超出窗口
        const menuWidth = 160;
        const menuHeight = 100;
        const x = Math.min(e.clientX, window.innerWidth - menuWidth);
        const y = Math.min(e.clientY, window.innerHeight - menuHeight);
        setMenuPos({ x: Math.max(0, x), y: Math.max(0, y) });
        setMenuOpen(true);
    };

    const handleQuit = async () => {
        try {
            await fetch('http://127.0.0.1:56789/action', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action: 'exit' })
            });
        } catch (e) {
            console.error("Failed to send exit command", e);
        }
    };

    const handleSettings = () => {
        // TODO: implement settings
    };

    // 截断过长文本用于显示
    const displayText = text && text.length > 200 ? text.slice(0, 200) + '…' : text;

    return (
        <div
            className="flex items-center justify-center relative"
            onContextMenu={handleContextMenu}
        >
            <ContextMenu
                x={menuPos.x}
                y={menuPos.y}
                isOpen={menuOpen}
                onClose={() => setMenuOpen(false)}
                onQuit={handleQuit}
                onSettings={handleSettings}
            />

            <motion.div
                layout
                data-tauri-drag-region
                initial={{ width: 48, height: 48, borderRadius: 24 }}
                animate={{
                    width: state === 'idle' || state === 'disconnected' ? 48
                        : state === 'error' ? 280
                        : (state === 'result' && displayText && displayText.length > 20 ? 'auto' : 320),
                    height: state === 'result' && displayText && displayText.length > 50 ? 'auto' : 48,
                    borderRadius: 24,
                    borderColor: state === 'disconnected' ? 'rgba(239, 68, 68, 0.3)'
                        : state === 'error' ? 'rgba(245, 158, 11, 0.3)'
                        : 'rgba(255, 255, 255, 0.1)'
                }}
                transition={{ type: "spring", stiffness: 400, damping: 25 }}
                className="glass-panel relative flex items-center justify-center overflow-hidden border"
                style={{
                    minWidth: 48, minHeight: 48, maxWidth: 600,
                    backgroundColor: state === 'disconnected' ? 'rgba(0,0,0,0.6)' : undefined
                }}
            >
                {/* 边框光晕 */}
                <div className={`absolute inset-0 rounded-full border pointer-events-none ${
                    state === 'disconnected' ? 'border-red-500/20'
                    : state === 'error' ? 'border-amber-500/20'
                    : 'border-white/10'
                }`} />

                <AnimatePresence mode="wait">

                    {/* DISCONNECTED */}
                    {state === 'disconnected' && (
                        <motion.div
                            key="disconnected"
                            initial={{ opacity: 0, scale: 0.5 }}
                            animate={{ opacity: 1, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.5 }}
                            className="text-red-400"
                        >
                            <WifiOff className="w-5 h-5" />
                        </motion.div>
                    )}

                    {/* ERROR */}
                    {state === 'error' && (
                        <motion.div
                            key="error"
                            initial={{ opacity: 0, scale: 0.5 }}
                            animate={{ opacity: 1, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.5 }}
                            className="flex items-center space-x-2 px-4"
                        >
                            <div className="bg-amber-500/20 p-1.5 rounded-full shrink-0">
                                <AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
                            </div>
                            <span className="text-amber-300 text-xs font-medium truncate">
                                识别失败
                            </span>
                        </motion.div>
                    )}

                    {/* EXITING */}
                    {state === 'exiting' && (
                        <motion.div
                            key="exiting"
                            initial={{ opacity: 0, scale: 0.5 }}
                            animate={{ opacity: 1, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.5 }}
                            className="flex items-center space-x-2 text-rose-400 font-medium text-sm"
                        >
                            <span>👋 再见!</span>
                        </motion.div>
                    )}

                    {/* IDLE */}
                    {state === 'idle' && (
                        <motion.div
                            key="idle"
                            initial={{ opacity: 0, scale: 0.5 }}
                            exit={{ opacity: 0, scale: 0.5 }}
                            className="w-2 h-2 rounded-full bg-cyan-500 shadow-[0_0_10px_rgba(6,182,212,0.8)]"
                            animate={{ opacity: [0.5, 1, 0.5], scale: 1 }}
                            transition={{ duration: 2, repeat: Infinity }}
                        />
                    )}

                    {/* LISTENING */}
                    {state === 'listening' && (
                        <motion.div
                            key="listening"
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="flex items-center space-x-1 h-4"
                        >
                            <Mic className="w-4 h-4 text-cyan-400 mr-3" />
                            {waveform.map((h, i) => (
                                <motion.div
                                    key={i}
                                    className="w-1 bg-cyan-400 rounded-full"
                                    animate={{
                                        height: [4, h, 4],
                                        opacity: [0.5, 1, 0.5]
                                    }}
                                    transition={{
                                        repeat: Infinity,
                                        duration: 0.5 + Math.random() * 0.5,
                                        delay: i * 0.05,
                                        ease: "easeInOut"
                                    }}
                                />
                            ))}
                        </motion.div>
                    )}

                    {/* PROCESSING */}
                    {state === 'processing' && (
                        <motion.div
                            key="processing"
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="flex items-center space-x-3 text-cyan-500 font-mono text-xs tracking-widest"
                        >
                            <motion.div
                                animate={{ rotate: 360 }}
                                transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
                            >
                                <Zap className="w-4 h-4" />
                            </motion.div>
                            <span>识别中...</span>
                        </motion.div>
                    )}

                    {/* RESULT */}
                    {state === 'result' && (
                        <motion.div
                            key="result"
                            initial={{ opacity: 0, y: 10 }}
                            animate={{ opacity: 1, y: 0 }}
                            exit={{ opacity: 0, y: -10 }}
                            className="px-6 py-3 flex items-center w-full"
                        >
                            <div className="bg-emerald-500/20 p-1.5 rounded-full mr-3 shrink-0">
                                <Check className="w-3 h-3 text-emerald-400" />
                            </div>
                            <span className="text-white text-sm font-medium leading-relaxed drop-shadow-md selection:bg-cyan-500/30">
                                {displayText}
                            </span>
                        </motion.div>
                    )}
                </AnimatePresence>
            </motion.div>
        </div>
    );
}
