import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { appWindow, LogicalSize, LogicalPosition, currentMonitor } from '@tauri-apps/api/window';
import { motion, AnimatePresence } from 'framer-motion';
import { Check, AlertTriangle, Power } from 'lucide-react';

interface HUDProps {
    state: 'starting' | 'idle' | 'listening' | 'processing' | 'result' | 'disconnected' | 'exiting' | 'error';
    text?: string;
}

const spring = { type: "spring" as const, stiffness: 500, damping: 32 };
const fade = {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
    transition: { duration: 0.15 },
};

const PILL_H = 48;
const PAD = 16;
const MENU_PILL_W = 170; // pill width when showing inline menu

export function HUD({ state, text }: HUDProps) {
    const [menuOpen, setMenuOpen] = useState(false);

    const displayText = text && text.length > 100 ? text.slice(0, 100) + '\u2026' : text;

    // Auto-close menu when state transitions to an active phase
    // (e.g., user presses shortcut while menu is open)
    useEffect(() => {
        if (menuOpen && (state === 'listening' || state === 'processing' || state === 'exiting')) {
            setMenuOpen(false);
        }
    }, [state, menuOpen]);

    const pillWidth = menuOpen ? MENU_PILL_W
        : state === 'idle' || state === 'disconnected' ? 48
        : state === 'starting' ? 160
        : state === 'listening' ? 220
        : state === 'processing' ? 170
        : state === 'error' ? 200
        : state === 'exiting' ? 100
        : state === 'result' ? Math.min(Math.max(180, (displayText?.length || 0) * 11 + 70), 440)
        : 48;

    // --- Dynamic window sizing (width only, height constant) ---
    const initRef = useRef(false);
    const prevW = useRef(pillWidth);
    const resizeId = useRef(0); // monotonic counter to cancel stale async resizes

    useEffect(() => {
        const w = pillWidth + PAD;
        const h = PILL_H + PAD;

        const was = prevW.current;
        prevW.current = pillWidth;
        const delay = pillWidth < was ? 250 : 0;

        const id = ++resizeId.current;

        const timer = setTimeout(async () => {
            if (resizeId.current !== id) return; // superseded by a newer resize
            try {
                if (!initRef.current) {
                    initRef.current = true;
                    const monitor = await currentMonitor();
                    if (!monitor || resizeId.current !== id) return;
                    const sf = monitor.scaleFactor;
                    const sw = monitor.size.width / sf;
                    const sh = monitor.size.height / sf;
                    await Promise.all([
                        appWindow.setSize(new LogicalSize(w, h)),
                        appWindow.setPosition(new LogicalPosition(
                            Math.round((sw - w) / 2),
                            Math.round(sh - h - 80),
                        )),
                    ]);
                } else {
                    const [pos, size, sf] = await Promise.all([
                        appWindow.outerPosition(),
                        appWindow.outerSize(),
                        appWindow.scaleFactor(),
                    ]);
                    if (resizeId.current !== id) return; // superseded
                    const oldW = size.width / sf;
                    const oldX = pos.x / sf;
                    const oldY = pos.y / sf;
                    await Promise.all([
                        appWindow.setSize(new LogicalSize(w, h)),
                        appWindow.setPosition(new LogicalPosition(
                            Math.round(oldX - (w - oldW) / 2),
                            oldY,
                        )),
                    ]);
                }
            } catch (e) {
                console.error('resize', e);
            }
        }, delay);

        return () => clearTimeout(timer);
    }, [pillWidth]);

    // --- Mouse handling ---
    const menuRef = useRef(menuOpen);
    menuRef.current = menuOpen;

    // Track active drag listeners for cleanup on unmount
    const dragCleanupRef = useRef<(() => void) | null>(null);

    useEffect(() => {
        return () => { dragCleanupRef.current?.(); };
    }, []);

    const handleMouseDown = (e: React.MouseEvent) => {
        if (e.button !== 0) return;
        if (menuRef.current) {
            setMenuOpen(false);
            return;
        }

        // Don't call startDragging() immediately — wait for actual mouse movement.
        // Immediate startDragging() captures the mouse and swallows contextmenu
        // events, which breaks trackpad two-finger right-click.
        dragCleanupRef.current?.(); // cancel any previous drag session
        const sx = e.screenX, sy = e.screenY;
        const onMove = (ev: MouseEvent) => {
            if (Math.abs(ev.screenX - sx) > 3 || Math.abs(ev.screenY - sy) > 3) {
                cleanup();
                appWindow.startDragging();
            }
        };
        const onUp = () => cleanup();
        const cleanup = () => {
            dragCleanupRef.current = null;
            document.removeEventListener('mousemove', onMove);
            document.removeEventListener('mouseup', onUp);
        };
        dragCleanupRef.current = cleanup;
        document.addEventListener('mousemove', onMove);
        document.addEventListener('mouseup', onUp);
    };

    // Auto-close menu after 3s
    useEffect(() => {
        if (!menuOpen) return;
        const t = setTimeout(() => setMenuOpen(false), 3000);
        return () => clearTimeout(t);
    }, [menuOpen]);

    const handleContextMenu = (e: React.MouseEvent) => {
        e.preventDefault();
        setMenuOpen(prev => !prev);
    };

    const handleQuit = (e: React.MouseEvent) => {
        if (e.button !== 0) return; // only left click triggers quit
        e.stopPropagation();
        setMenuOpen(false);
        invoke('quit_app').catch(console.error);
    };

    return (
        <div onMouseDown={handleMouseDown} onContextMenu={handleContextMenu}>
            <motion.div
                animate={{ width: pillWidth, borderRadius: 24 }}
                transition={spring}
                className="glass-pill h-12 flex items-center justify-center overflow-hidden cursor-default select-none"
                style={{ minWidth: 48 }}
            >
                <AnimatePresence mode="wait">
                    {/* INLINE MENU */}
                    {menuOpen && (
                        <motion.div key="menu" {...fade}
                            className="flex items-center px-4 w-full"
                            onMouseDown={(e) => e.stopPropagation()}
                            onContextMenu={(e) => e.stopPropagation()}
                        >
                            <button
                                onMouseDown={handleQuit}
                                className="text-xs text-rose-400 hover:text-rose-300 flex items-center gap-2"
                            >
                                <Power className="w-3.5 h-3.5" />
                                退出 FlashSpeech
                            </button>
                        </motion.div>
                    )}

                    {/* IDLE */}
                    {!menuOpen && state === 'idle' && (
                        <motion.div key="idle" {...fade}>
                            <motion.div
                                className="w-2.5 h-2.5 rounded-full bg-white/80"
                                animate={{ opacity: [0.4, 0.9, 0.4], scale: [0.85, 1, 0.85] }}
                                transition={{ duration: 2.5, repeat: Infinity, ease: "easeInOut" }}
                            />
                        </motion.div>
                    )}

                    {/* STARTING */}
                    {!menuOpen && state === 'starting' && (
                        <motion.div key="starting" {...fade}
                            className="flex items-center gap-2.5 px-4 text-white/60 text-xs tracking-wide"
                        >
                            <motion.div className="w-1.5 h-1.5 rounded-full bg-white/50"
                                animate={{ opacity: [0.3, 1, 0.3] }}
                                transition={{ duration: 1.2, repeat: Infinity }}
                            />
                            <span>启动中</span>
                        </motion.div>
                    )}

                    {/* LISTENING */}
                    {!menuOpen && state === 'listening' && (
                        <motion.div key="listening" {...fade}
                            className="flex items-center gap-3 px-4"
                        >
                            <motion.div className="w-2 h-2 rounded-full bg-red-400 shrink-0"
                                animate={{ opacity: [1, 0.4, 1], scale: [1, 0.8, 1] }}
                                transition={{ duration: 1, repeat: Infinity }}
                            />
                            <div className="flex items-center gap-[3px] h-5">
                                {Array.from({ length: 8 }, (_, i) => (
                                    <motion.div
                                        key={i}
                                        className="w-[3px] rounded-full bg-white/70"
                                        animate={{ height: [3, 8 + Math.random() * 12, 3] }}
                                        transition={{
                                            repeat: Infinity,
                                            duration: 0.4 + Math.random() * 0.4,
                                            delay: i * 0.06,
                                            ease: "easeInOut",
                                        }}
                                    />
                                ))}
                            </div>
                        </motion.div>
                    )}

                    {/* PROCESSING */}
                    {!menuOpen && state === 'processing' && (
                        <motion.div key="processing" {...fade}
                            className="flex items-center gap-2.5 px-4 text-white/60 text-xs tracking-wide"
                        >
                            <motion.div
                                animate={{ rotate: 360 }}
                                transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                                className="w-4 h-4 border-2 border-white/20 border-t-white/70 rounded-full"
                            />
                            <span>识别中</span>
                        </motion.div>
                    )}

                    {/* RESULT */}
                    {!menuOpen && state === 'result' && displayText && (
                        <motion.div key="result" {...fade}
                            className="flex items-center gap-2 px-4 w-full"
                        >
                            <Check className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                            <span className="text-white/90 text-[13px] leading-tight truncate">
                                {displayText}
                            </span>
                        </motion.div>
                    )}

                    {/* ERROR */}
                    {!menuOpen && state === 'error' && (
                        <motion.div key="error" {...fade}
                            className="flex items-center gap-2 px-4 text-amber-400/80 text-xs"
                        >
                            <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                            <span>识别失败</span>
                        </motion.div>
                    )}

                    {/* DISCONNECTED */}
                    {!menuOpen && state === 'disconnected' && (
                        <motion.div key="disconnected" {...fade}>
                            <div className="w-2.5 h-2.5 rounded-full bg-red-400/60" />
                        </motion.div>
                    )}

                    {/* EXITING */}
                    {!menuOpen && state === 'exiting' && (
                        <motion.div key="exiting" {...fade}
                            className="text-white/50 text-xs"
                        >
                            再见
                        </motion.div>
                    )}
                </AnimatePresence>
            </motion.div>
        </div>
    );
}
