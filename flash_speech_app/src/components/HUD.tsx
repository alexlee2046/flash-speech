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
    initial: { opacity: 0, scale: 0.96 },
    animate: { opacity: 1, scale: 1 },
    exit: { opacity: 0, scale: 0.96 },
    transition: { duration: 0.18, ease: [0.22, 1, 0.36, 1] },
};

const PILL_H = 44;
const PAD = 16;
const MENU_PILL_W = 160;

/**
 * SVG Filter 定义 — 微妙的光学折射扭曲
 */
function LiquidGlassFilters() {
    return (
        <svg width="0" height="0" style={{ position: 'absolute' }}>
            <defs>
                <filter id="liquid-refraction" x="-5%" y="-5%" width="110%" height="110%">
                    <feTurbulence
                        type="fractalNoise"
                        baseFrequency="0.015"
                        numOctaves="3"
                        seed="2"
                        result="noise"
                    />
                    <feGaussianBlur in="noise" stdDeviation="3" result="smoothNoise" />
                    <feDisplacementMap
                        in="SourceGraphic"
                        in2="smoothNoise"
                        scale="2"
                        xChannelSelector="R"
                        yChannelSelector="G"
                    />
                </filter>
            </defs>
        </svg>
    );
}

/* 使用 CSS 变量的颜色类名 — 自动适配亮色/暗色模式 */
const textStyle = { color: 'var(--glass-text)' } as React.CSSProperties;
const textSecondaryStyle = { color: 'var(--glass-text-secondary)' } as React.CSSProperties;
const dotStyle = { background: 'var(--glass-dot)' } as React.CSSProperties;
const barStyle = { background: 'var(--glass-bar)' } as React.CSSProperties;

export function HUD({ state, text }: HUDProps) {
    const [menuOpen, setMenuOpen] = useState(false);
    const [ripple, setRipple] = useState<{ x: number; y: number; id: number } | null>(null);
    const [flash, setFlash] = useState(false);
    const prevState = useRef(state);

    const displayText = text && text.length > 100 ? text.slice(0, 100) + '\u2026' : text;

    // ── 高光旋转 (10s 周期) ──
    const highlightAngle = useMotionValue(135);
    useEffect(() => {
        const controls = animate(highlightAngle, 135 + 360, {
            duration: 10,
            repeat: Infinity,
            ease: "linear",
        });
        return controls.stop;
    }, []);

    const highlightBg = useTransform(highlightAngle, (v) =>
        `linear-gradient(${v}deg, var(--glass-highlight-top) 0%, rgba(255,255,255,0.08) 35%, transparent 60%)`
    );

    // ── 边框光环旋转 ──
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
            ? `conic-gradient(from ${v}deg, rgba(255,80,80,0.65), rgba(255,130,110,0.5), rgba(255,80,80,0.65), rgba(255,60,60,0.5), rgba(255,80,80,0.65))`
            : `conic-gradient(from ${v}deg, rgba(255,255,255,0.25), rgba(200,220,255,0.2), rgba(255,200,180,0.18), rgba(200,255,220,0.18), rgba(180,200,255,0.2), rgba(255,255,255,0.25))`
    );

    // ── 状态过渡闪光 ──
    useEffect(() => {
        if (prevState.current !== state) {
            prevState.current = state;
            setFlash(true);
            const t = setTimeout(() => setFlash(false), 150);
            return () => clearTimeout(t);
        }
    }, [state]);

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

    // ── 动态窗口大小调整 ──
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

    // ── 鼠标交互 ──
    const menuRef = useRef(menuOpen);
    menuRef.current = menuOpen;
    const dragCleanupRef = useRef<(() => void) | null>(null);

    useEffect(() => {
        return () => { dragCleanupRef.current?.(); };
    }, []);

    const handleMouseDown = (e: React.MouseEvent) => {
        if (e.button !== 0) return;
        if (menuRef.current) { setMenuOpen(false); return; }

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

    useEffect(() => {
        if (!ripple) return;
        const t = setTimeout(() => setRipple(null), 350);
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
            <LiquidGlassFilters />

            <motion.div
                animate={{ width: pillWidth, borderRadius: 22 }}
                transition={spring}
                className="liquid-glass h-[44px] flex items-center justify-center cursor-default select-none"
                style={{ minWidth: 44 }}
            >
                {/* 高光扫过层 */}
                <motion.div className="liquid-glass-highlight" style={{ background: highlightBg }} />

                {/* 折射模拟层 */}
                <div className="liquid-glass-refraction" />

                {/* 噪点纹理层 */}
                <div className="liquid-glass-noise" />

                {/* 边框光环层 */}
                <motion.div
                    className={`liquid-glass-rim ${state === 'listening' ? 'listening' : ''}`}
                    style={{ background: rimBg }}
                />

                {/* 闪光层 */}
                <div className={`liquid-glass-flash ${flash ? 'active' : ''}`} />

                {/* 涟漪层 */}
                {ripple && (
                    <div key={ripple.id} className="liquid-glass-ripple active"
                        style={{ left: ripple.x, top: ripple.y }} />
                )}

                {/* ── 内容层 — 使用 CSS 变量颜色自适应暗色模式 ── */}
                <AnimatePresence mode="wait">
                    {/* 菜单 */}
                    {menuOpen && (
                        <motion.div key="menu" {...fade}
                            className="relative z-10 flex items-center px-4 w-full"
                            onMouseDown={(e) => e.stopPropagation()}
                            onContextMenu={(e) => e.stopPropagation()}
                        >
                            <button
                                onMouseDown={handleQuit}
                                className="text-xs flex items-center gap-2 transition-colors"
                                style={{ color: 'rgba(255,80,80,0.9)' }}
                            >
                                <Power className="w-3.5 h-3.5" />
                                退出 FlashSpeech
                            </button>
                        </motion.div>
                    )}

                    {/* 空闲 */}
                    {!menuOpen && state === 'idle' && (
                        <motion.div key="idle" {...fade} className="relative z-10">
                            <motion.div
                                className="w-2.5 h-2.5 rounded-full"
                                style={dotStyle}
                                animate={{ opacity: [0.4, 0.9, 0.4], scale: [0.85, 1, 0.85] }}
                                transition={{ duration: 2.5, repeat: Infinity, ease: "easeInOut" }}
                            />
                        </motion.div>
                    )}

                    {/* 启动中 */}
                    {!menuOpen && state === 'starting' && (
                        <motion.div key="starting" {...fade}
                            className="relative z-10 flex items-center gap-2.5 px-4 text-xs tracking-wide"
                            style={textSecondaryStyle}
                        >
                            <motion.div className="w-1.5 h-1.5 rounded-full"
                                style={dotStyle}
                                animate={{ opacity: [0.3, 1, 0.3] }}
                                transition={{ duration: 1.2, repeat: Infinity }}
                            />
                            <span>启动中</span>
                        </motion.div>
                    )}

                    {/* 监听中 */}
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
                                        className="w-[3px] rounded-full"
                                        style={barStyle}
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

                    {/* 处理中 */}
                    {!menuOpen && state === 'processing' && (
                        <motion.div key="processing" {...fade}
                            className="relative z-10 flex items-center gap-2.5 px-4 text-xs tracking-wide"
                            style={textSecondaryStyle}
                        >
                            <motion.div
                                animate={{ rotate: 360 }}
                                transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                                className="w-4 h-4 rounded-full"
                                style={{
                                    border: '2px solid var(--glass-highlight-inner)',
                                    borderTopColor: 'var(--glass-text)',
                                }}
                            />
                            <span>识别中</span>
                        </motion.div>
                    )}

                    {/* 结果 */}
                    {!menuOpen && state === 'result' && displayText && (
                        <motion.div key="result" {...fade}
                            className="relative z-10 flex items-center gap-2 px-4 w-full"
                        >
                            <Check className="w-3.5 h-3.5 shrink-0" style={{ color: '#34d399' }} />
                            <span className="text-[13px] leading-tight truncate" style={textStyle}>
                                {displayText}
                            </span>
                        </motion.div>
                    )}

                    {/* 错误 */}
                    {!menuOpen && state === 'error' && (
                        <motion.div key="error" {...fade}
                            className="relative z-10 flex items-center gap-2 px-4 text-xs"
                            style={{ color: '#fbbf24' }}
                        >
                            <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                            <span>识别失败</span>
                        </motion.div>
                    )}

                    {/* 断开连接 */}
                    {!menuOpen && state === 'disconnected' && (
                        <motion.div key="disconnected" {...fade} className="relative z-10">
                            <div className="w-2.5 h-2.5 rounded-full bg-red-500/50" />
                        </motion.div>
                    )}

                    {/* 退出中 */}
                    {!menuOpen && state === 'exiting' && (
                        <motion.div key="exiting" {...fade}
                            className="relative z-10 text-xs"
                            style={textSecondaryStyle}
                        >
                            再见
                        </motion.div>
                    )}
                </AnimatePresence>
            </motion.div>
        </div>
    );
}
