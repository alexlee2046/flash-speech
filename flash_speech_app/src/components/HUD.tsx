import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { appWindow, LogicalSize, LogicalPosition, currentMonitor } from '@tauri-apps/api/window';
import { motion, AnimatePresence, useMotionValue, useTransform, animate } from 'framer-motion';
import { Check, AlertTriangle, Power } from 'lucide-react';

interface HUDProps {
    state: 'starting' | 'idle' | 'listening' | 'processing' | 'result' | 'disconnected' | 'exiting' | 'error';
    text?: string;
}

const spring = { type: "spring" as const, stiffness: 400, damping: 28 };
const fade = {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
    transition: { duration: 0.15 },
};

const PILL_H = 44;
const PAD = 16;
const MENU_PILL_W = 160;

export function HUD({ state, text }: HUDProps) {
    const [menuOpen, setMenuOpen] = useState(false);
    const [ripple, setRipple] = useState<{ x: number; y: number; id: number } | null>(null);
    const [flash, setFlash] = useState(false);
    const prevState = useRef(state);

    const displayText = text && text.length > 100 ? text.slice(0, 100) + '\u2026' : text;

    // Highlight rotation (8s cycle) — JS-driven for WKWebView compat
    const highlightAngle = useMotionValue(135);
    useEffect(() => {
        const controls = animate(highlightAngle, 135 + 360, {
            duration: 8,
            repeat: Infinity,
            ease: "linear",
        });
        return controls.stop;
    }, []);

    const highlightBg = useTransform(highlightAngle, (v) =>
        `linear-gradient(${v}deg, rgba(255,255,255,0.55) 0%, rgba(255,255,255,0.08) 50%, transparent 100%)`
    );

    // Rim rotation (6s cycle, 3s when listening) — JS-driven for WKWebView compat
    const rimAngle = useMotionValue(0);
    const rimDuration = useRef(6);
    const rimControls = useRef<{ stop: () => void } | null>(null);

    useEffect(() => {
        const newDuration = state === 'listening' ? 3 : 6;
        if (newDuration !== rimDuration.current) {
            rimDuration.current = newDuration;
            rimControls.current?.stop();
            const current = rimAngle.get() % 360;
            rimAngle.set(current);
            rimControls.current = animate(rimAngle, current + 360, {
                duration: newDuration,
                repeat: Infinity,
                ease: "linear",
            });
        }
        if (!rimControls.current) {
            rimControls.current = animate(rimAngle, 360, {
                duration: 6,
                repeat: Infinity,
                ease: "linear",
            });
        }
        return () => { rimControls.current?.stop(); };
    }, [state]);

    const rimBg = useTransform(rimAngle, (v) =>
        state === 'listening'
            ? `conic-gradient(from ${v}deg, rgba(255,80,80,0.6), rgba(255,120,100,0.5), rgba(255,80,80,0.6), rgba(255,60,60,0.5), rgba(255,80,80,0.6))`
            : `conic-gradient(from ${v}deg, rgba(255,120,120,0.4), rgba(255,200,100,0.4), rgba(100,255,150,0.4), rgba(100,180,255,0.4), rgba(200,130,255,0.4), rgba(255,120,120,0.4))`
    );

    // Flash on state transitions
    useEffect(() => {
        if (prevState.current !== state) {
            prevState.current = state;
            setFlash(true);
            const t = setTimeout(() => setFlash(false), 150);
            return () => clearTimeout(t);
        }
    }, [state]);

    // Auto-close menu when state transitions to an active phase
    useEffect(() => {
        if (menuOpen && (state === 'listening' || state === 'processing' || state === 'exiting')) {
            setMenuOpen(false);
        }
    }, [state, menuOpen]);

    const pillWidth = menuOpen ? MENU_PILL_W
        : state === 'idle' || state === 'disconnected' ? 44
        : state === 'starting' ? 140
        : state === 'listening' ? 200
        : state === 'processing' ? 160
        : state === 'error' ? 180
        : state === 'exiting' ? 80
        : state === 'result' ? Math.min(Math.max(180, (displayText?.length || 0) * 11 + 70), 420)
        : 44;

    // --- Dynamic window sizing ---
    const initRef = useRef(false);
    const prevW = useRef(pillWidth);
    const resizeId = useRef(0);

    useEffect(() => {
        const w = pillWidth + PAD;
        const h = PILL_H + PAD;

        const was = prevW.current;
        prevW.current = pillWidth;
        const delay = pillWidth < was ? 250 : 0;

        const id = ++resizeId.current;

        const timer = setTimeout(async () => {
            if (resizeId.current !== id) return;
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
                    if (resizeId.current !== id) return;
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

        dragCleanupRef.current?.();
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

    const handleContextMenu = useCallback((e: React.MouseEvent) => {
        e.preventDefault();
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        setRipple({ x: e.clientX - rect.left, y: e.clientY - rect.top, id: Date.now() });
        setMenuOpen(prev => !prev);
    }, []);

    // Clear ripple after animation
    useEffect(() => {
        if (!ripple) return;
        const t = setTimeout(() => setRipple(null), 300);
        return () => clearTimeout(t);
    }, [ripple]);

    const handleQuit = (e: React.MouseEvent) => {
        if (e.button !== 0) return;
        e.stopPropagation();
        setMenuOpen(false);
        invoke('quit_app').catch(console.error);
    };

    return (
        <div onMouseDown={handleMouseDown} onContextMenu={handleContextMenu}>
            <motion.div
                animate={{ width: pillWidth, borderRadius: 22 }}
                transition={spring}
                className="liquid-glass h-[44px] flex items-center justify-center cursor-default select-none"
                style={{ minWidth: 44 }}
            >
                {/* Highlight layer */}
                <motion.div className="liquid-glass-highlight" style={{ background: highlightBg }} />

                {/* Rim light layer */}
                <motion.div className={`liquid-glass-rim ${state === 'listening' ? 'listening' : ''}`} style={{ background: rimBg }} />

                {/* Flash layer */}
                <div className={`liquid-glass-flash ${flash ? 'active' : ''}`} />

                {/* Ripple layer */}
                {ripple && (
                    <div
                        key={ripple.id}
                        className="liquid-glass-ripple active"
                        style={{ left: ripple.x, top: ripple.y }}
                    />
                )}

                {/* Content layer */}
                <AnimatePresence mode="wait">
                    {/* INLINE MENU */}
                    {menuOpen && (
                        <motion.div key="menu" {...fade}
                            className="relative z-10 flex items-center px-4 w-full"
                            onMouseDown={(e) => e.stopPropagation()}
                            onContextMenu={(e) => e.stopPropagation()}
                        >
                            <button
                                onMouseDown={handleQuit}
                                className="text-xs text-rose-600 hover:text-rose-500 flex items-center gap-2 transition-colors"
                            >
                                <Power className="w-3.5 h-3.5" />
                                退出 FlashSpeech
                            </button>
                        </motion.div>
                    )}

                    {/* IDLE */}
                    {!menuOpen && state === 'idle' && (
                        <motion.div key="idle" {...fade} className="relative z-10">
                            <motion.div
                                className="w-2.5 h-2.5 rounded-full bg-gray-800/60"
                                animate={{ opacity: [0.4, 0.9, 0.4], scale: [0.85, 1, 0.85] }}
                                transition={{ duration: 2.5, repeat: Infinity, ease: "easeInOut" }}
                            />
                        </motion.div>
                    )}

                    {/* STARTING */}
                    {!menuOpen && state === 'starting' && (
                        <motion.div key="starting" {...fade}
                            className="relative z-10 flex items-center gap-2.5 px-4 text-gray-600 text-xs tracking-wide"
                        >
                            <motion.div className="w-1.5 h-1.5 rounded-full bg-gray-500"
                                animate={{ opacity: [0.3, 1, 0.3] }}
                                transition={{ duration: 1.2, repeat: Infinity }}
                            />
                            <span>启动中</span>
                        </motion.div>
                    )}

                    {/* LISTENING */}
                    {!menuOpen && state === 'listening' && (
                        <motion.div key="listening" {...fade}
                            className="relative z-10 flex items-center gap-3 px-4"
                        >
                            <motion.div className="w-2 h-2 rounded-full bg-red-500 shrink-0"
                                animate={{ opacity: [1, 0.4, 1], scale: [1, 0.8, 1] }}
                                transition={{ duration: 1, repeat: Infinity }}
                            />
                            <div className="flex items-center gap-[3px] h-5">
                                {Array.from({ length: 8 }, (_, i) => (
                                    <motion.div
                                        key={i}
                                        className="w-[3px] rounded-full bg-gray-700/70"
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
                            className="relative z-10 flex items-center gap-2.5 px-4 text-gray-600 text-xs tracking-wide"
                        >
                            <motion.div
                                animate={{ rotate: 360 }}
                                transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                                className="w-4 h-4 border-2 border-gray-400/30 border-t-gray-700/80 rounded-full"
                            />
                            <span>识别中</span>
                        </motion.div>
                    )}

                    {/* RESULT */}
                    {!menuOpen && state === 'result' && displayText && (
                        <motion.div key="result" {...fade}
                            className="relative z-10 flex items-center gap-2 px-4 w-full"
                        >
                            <Check className="w-3.5 h-3.5 text-emerald-600 shrink-0" />
                            <span className="text-gray-800/90 text-[13px] leading-tight truncate">
                                {displayText}
                            </span>
                        </motion.div>
                    )}

                    {/* ERROR */}
                    {!menuOpen && state === 'error' && (
                        <motion.div key="error" {...fade}
                            className="relative z-10 flex items-center gap-2 px-4 text-amber-600 text-xs"
                        >
                            <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                            <span>识别失败</span>
                        </motion.div>
                    )}

                    {/* DISCONNECTED */}
                    {!menuOpen && state === 'disconnected' && (
                        <motion.div key="disconnected" {...fade} className="relative z-10">
                            <div className="w-2.5 h-2.5 rounded-full bg-red-500/50" />
                        </motion.div>
                    )}

                    {/* EXITING */}
                    {!menuOpen && state === 'exiting' && (
                        <motion.div key="exiting" {...fade}
                            className="relative z-10 text-gray-500 text-xs"
                        >
                            再见
                        </motion.div>
                    )}
                </AnimatePresence>
            </motion.div>
        </div>
    );
}
