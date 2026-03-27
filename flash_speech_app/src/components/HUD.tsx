import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { motion, AnimatePresence } from 'framer-motion';
import { Check, AlertTriangle, Power, LayoutGrid, Settings, Layers, Circle, CheckCircle2, Loader2, Globe } from 'lucide-react';

import { useWindowDynamicSize } from '../hooks/useWindowDynamicSize';
import { useWindowDrag } from '../hooks/useWindowDrag';

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
const MENU_PILL_W = 200;
const MENU_PILL_H = 360;

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
    const [activeModel, setActiveModel] = useState<'sensevoice' | 'whisper' | 'paraformer'>(() => {
        return (localStorage.getItem('flashspeech_model') as 'sensevoice' | 'whisper' | 'paraformer') || 'sensevoice';
    });
    const [switching, setSwitching] = useState(false);
    const [activeLanguage, setActiveLanguage] = useState<string>(() => {
        return localStorage.getItem('flashspeech_language') || 'auto';
    });
    const [switchingLang, setSwitchingLang] = useState(false);
    const prevState = useRef(state);

    const displayText = text && text.length > 100 ? text.slice(0, 100) + '\u2026' : text;

    // ── 状态过渡闪光 ──
    useEffect(() => {
        if (prevState.current !== state) {
            prevState.current = state;
            setFlash(true);
            const t = setTimeout(() => setFlash(false), 150);
            return () => clearTimeout(t);
        }
    }, [state]);

    // ── 自动收起菜单 ──
    useEffect(() => {
        if (menuOpen && (state === 'listening' || state === 'processing' || state === 'exiting')) {
            setMenuOpen(false);
        }
    }, [state, menuOpen]);

    useEffect(() => {
        if (!menuOpen) return;
        // 增加菜单显示时间以方便操作
        const t = setTimeout(() => setMenuOpen(false), 5000);
        return () => clearTimeout(t);
    }, [menuOpen]);

    // ── 目标尺寸计算 ──
    const pillWidth = menuOpen ? MENU_PILL_W
        : state === 'idle' ? 140
            : state === 'disconnected' ? 44
                : state === 'starting' ? 140
                    : state === 'listening' ? 200
                        : state === 'processing' ? 160
                            : state === 'error' ? 180
                                : state === 'exiting' ? 80
                                    : state === 'result' ? Math.min(Math.max(180, (displayText?.length || 0) * 11 + 70), 420)
                                        : 44;

    const pillHeight = menuOpen ? MENU_PILL_H : PILL_H;

    // ── 动态管理 Tauri 窗口尺寸和拖拽 (自定义 Hooks) ──
    useWindowDynamicSize(pillWidth + PAD, pillHeight + PAD);
    const { handleMouseDown: dragMouseDown } = useWindowDrag(menuOpen); // 菜单打开时禁用拖拽

    // ── 鼠标交互 ──
    const handleMouseDown = (e: React.MouseEvent) => {
        if (e.button !== 0) return;
        if (menuOpen) { setMenuOpen(false); return; }
        dragMouseDown(e);
    };

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

    const handleSwitchLanguage = async (lang: string) => {
        if (lang === activeLanguage || switchingLang) return;
        setSwitchingLang(true);
        try {
            await invoke('switch_language', { lang });
            setActiveLanguage(lang);
            localStorage.setItem('flashspeech_language', lang);
        } catch (e) {
            console.error('Failed to switch language:', e);
        } finally {
            setSwitchingLang(false);
        }
    };

    const handleSwitchModel = async (model: 'sensevoice' | 'whisper' | 'paraformer') => {
        if (model === activeModel || switching) return;
        setSwitching(true);
        try {
            await invoke('switch_model', { name: model });
            setActiveModel(model);
            localStorage.setItem('flashspeech_model', model);

            // Allow menu to show the updated checkmark briefly before closing
            setTimeout(() => setMenuOpen(false), 600);
        } catch (e) {
            console.error('Failed to switch model:', e);
        } finally {
            setSwitching(false);
        }
    };

    return (
        <div onMouseDown={handleMouseDown} onContextMenu={handleContextMenu}>
            <LiquidGlassFilters />

            <motion.div
                animate={{
                    width: pillWidth,
                    height: pillHeight,
                    borderRadius: menuOpen ? 24 : 22
                }}
                transition={spring}
                className="liquid-glass flex flex-col items-center justify-start cursor-default select-none overflow-hidden origin-bottom"
                style={{ minWidth: 44, minHeight: 44 }}
            >
                {/* 高光扫过层 (纯 CSS `@keyframes`) */}
                <div className="liquid-glass-highlight" />

                {/* 折射模拟层 */}
                <div className="liquid-glass-refraction" />

                {/* 噪点纹理层 */}
                <div className="liquid-glass-noise" />

                {/* 边框光环层 (纯 CSS `@keyframes`) */}
                <div className={`liquid-glass-rim ${state === 'listening' ? 'listening' : ''}`} />

                {/* 闪光层 */}
                <div className={`liquid-glass-flash ${flash ? 'active' : ''}`} />

                {/* 涟漪层 */}
                {ripple && (
                    <div key={ripple.id} className="liquid-glass-ripple active"
                        style={{ left: ripple.x, top: ripple.y }} />
                )}

                {/* ── 内容层 ── */}
                <div className="w-full h-full relative z-10">
                    <AnimatePresence mode="wait">
                        {/* 菜单 - Popover */}
                        {menuOpen && (
                            <motion.div key="menu" {...fade}
                                className="absolute inset-0 flex flex-col p-4 w-full h-full text-[13px]"
                                onMouseDown={(e) => e.stopPropagation()}
                                onContextMenu={(e) => e.stopPropagation()}
                            >
                                {/* Header / Title */}
                                <div className="flex items-center gap-2 mb-3 pb-3 border-b" style={{ borderColor: 'var(--glass-border)' }}>
                                    <LayoutGrid className="w-4 h-4 shrink-0" style={textStyle} />
                                    <span className="font-semibold tracking-wide" style={textStyle}>控制中心</span>
                                </div>

                                {/* Menu Items */}
                                <div className="flex flex-col gap-1.5 flex-1 w-full relative z-20">
                                    <div className="flex flex-col gap-1 px-1">
                                        <div className="flex items-center gap-1.5 pl-2 mb-1.5 mt-0.5 opacity-60">
                                            <Layers className="w-3.5 h-3.5" style={textStyle} />
                                            <span className="text-[11px] uppercase tracking-wider font-semibold" style={textStyle}>引擎模型</span>
                                        </div>

                                        <button
                                            onClick={() => handleSwitchModel('sensevoice')}
                                            className="flex items-center gap-2.5 px-2.5 py-1.5 rounded-[8px] transition-colors"
                                            style={activeModel === 'sensevoice' ? textStyle : textSecondaryStyle}
                                            onMouseEnter={(e) => e.currentTarget.style.backgroundColor = 'rgba(128,128,128,0.1)'}
                                            onMouseLeave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
                                            disabled={switching}
                                        >
                                            <div className="flex-1 text-left flex flex-col">
                                                <span className="font-semibold text-[13px] leading-tight mb-0.5">SenseVoice</span>
                                                <span className="text-[10px] opacity-70">极速多语种 (默认)</span>
                                            </div>
                                            {switching && activeModel !== 'sensevoice' ? null :
                                                activeModel === 'sensevoice' ? <CheckCircle2 className="w-[18px] h-[18px] text-green-500" /> : <Circle className="w-[18px] h-[18px] opacity-30" />
                                            }
                                        </button>

                                        <button
                                            onClick={() => handleSwitchModel('whisper')}
                                            className="flex items-center gap-2.5 px-2.5 py-1.5 rounded-[8px] transition-colors"
                                            style={activeModel === 'whisper' ? textStyle : textSecondaryStyle}
                                            onMouseEnter={(e) => e.currentTarget.style.backgroundColor = 'rgba(128,128,128,0.1)'}
                                            onMouseLeave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
                                            disabled={switching}
                                        >
                                            <div className="flex-1 text-left flex flex-col">
                                                <span className="font-semibold text-[13px] leading-tight mb-0.5">Whisper Tiny</span>
                                                <span className="text-[10px] opacity-70">高精纯英文 {switching && activeModel !== 'whisper' ? '(下载中...)' : ''}</span>
                                            </div>
                                            {switching && activeModel !== 'whisper' ? (
                                                <Loader2 className="w-[18px] h-[18px] animate-spin opacity-60 text-blue-500" />
                                            ) : (
                                                activeModel === 'whisper' ? <CheckCircle2 className="w-[18px] h-[18px] text-green-500" /> : <Circle className="w-[18px] h-[18px] opacity-30" />
                                            )}
                                        </button>

                                        <button
                                            onClick={() => handleSwitchModel('paraformer')}
                                            className="flex items-center gap-2.5 px-2.5 py-1.5 rounded-[8px] transition-colors"
                                            style={activeModel === 'paraformer' ? textStyle : textSecondaryStyle}
                                            onMouseEnter={(e) => e.currentTarget.style.backgroundColor = 'rgba(128,128,128,0.1)'}
                                            onMouseLeave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
                                            disabled={switching}
                                        >
                                            <div className="flex-1 text-left flex flex-col">
                                                <span className="font-semibold text-[13px] leading-tight mb-0.5">Paraformer</span>
                                                <span className="text-[10px] opacity-70">中英双语 {switching && activeModel !== 'paraformer' ? '(下载中...)' : ''}</span>
                                            </div>
                                            {switching && activeModel !== 'paraformer' ? (
                                                <Loader2 className="w-[18px] h-[18px] animate-spin opacity-60 text-blue-500" />
                                            ) : (
                                                activeModel === 'paraformer' ? <CheckCircle2 className="w-[18px] h-[18px] text-green-500" /> : <Circle className="w-[18px] h-[18px] opacity-30" />
                                            )}
                                        </button>
                                    </div>
                                    {/* ── 识别语言 ── */}
                                    <div className="flex flex-col gap-1 px-1 mt-2">
                                        <div className="flex items-center gap-1.5 pl-2 mb-1.5 opacity-60">
                                            <Globe className="w-3.5 h-3.5" style={textStyle} />
                                            <span className="text-[11px] uppercase tracking-wider font-semibold" style={textStyle}>识别语言</span>
                                            {switchingLang && <Loader2 className="w-3 h-3 animate-spin opacity-60 text-blue-500" />}
                                        </div>
                                        <div className="grid grid-cols-3 gap-1">
                                            {([
                                                ['auto', '自动'],
                                                ['zh', '中文'],
                                                ['en', 'EN'],
                                                ['ja', '日本語'],
                                                ['ko', '한국어'],
                                                ['yue', '粤语'],
                                            ] as const).map(([code, label]) => (
                                                <button
                                                    key={code}
                                                    onClick={() => handleSwitchLanguage(code)}
                                                    disabled={switchingLang}
                                                    className="px-2 py-1 rounded-[6px] text-[11px] font-medium transition-colors text-center"
                                                    style={{
                                                        ...(activeLanguage === code ? textStyle : textSecondaryStyle),
                                                        backgroundColor: activeLanguage === code ? 'rgba(52,211,153,0.15)' : 'transparent',
                                                        border: activeLanguage === code ? '1px solid rgba(52,211,153,0.3)' : '1px solid transparent',
                                                    }}
                                                    onMouseEnter={(e) => {
                                                        if (activeLanguage !== code) e.currentTarget.style.backgroundColor = 'rgba(128,128,128,0.1)';
                                                    }}
                                                    onMouseLeave={(e) => {
                                                        e.currentTarget.style.backgroundColor = activeLanguage === code ? 'rgba(52,211,153,0.15)' : 'transparent';
                                                    }}
                                                >
                                                    {label}
                                                </button>
                                            ))}
                                        </div>
                                    </div>
                                </div>

                                <div className="mt-auto pt-2">
                                    <button
                                        onMouseDown={handleQuit}
                                        className="flex items-center justify-center gap-2 w-full py-1.5 rounded-[10px] transition-all"
                                        style={{ color: 'rgba(255,80,80,0.9)' }}
                                        onMouseEnter={(e) => e.currentTarget.style.backgroundColor = 'rgba(255,80,80,0.08)'}
                                        onMouseLeave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
                                    >
                                        <Power className="w-3.5 h-3.5" />
                                        <span>退出应用</span>
                                    </button>
                                </div>
                            </motion.div>
                        )}

                        {/* 单行状态 - 确保在 44px 高度居中 */}
                        {!menuOpen && (
                            <div className="h-[44px] flex items-center justify-center w-full absolute top-0 left-0">
                                {/* 空闲 */}
                                {state === 'idle' && (
                                    <motion.div key="idle" {...fade}
                                        className="flex items-center gap-2.5 px-4 text-xs tracking-wide"
                                        style={textSecondaryStyle}
                                    >
                                        <motion.div
                                            className="w-1.5 h-1.5 rounded-full"
                                            style={dotStyle}
                                            animate={{ opacity: [0.3, 1, 0.3] }}
                                            transition={{ duration: 1.5, repeat: Infinity, ease: "easeInOut" }}
                                        />
                                        <span>Alt+Space 唤醒</span>
                                    </motion.div>
                                )}

                                {/* 启动中 */}
                                {state === 'starting' && (
                                    <motion.div key="starting" {...fade}
                                        className="flex items-center gap-2.5 px-4 text-xs tracking-wide"
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
                                {state === 'listening' && (
                                    <motion.div key="listening" {...fade}
                                        className="flex items-center gap-3 px-4"
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
                                {state === 'processing' && (
                                    <motion.div key="processing" {...fade}
                                        className="flex items-center gap-2.5 px-4 text-xs tracking-wide"
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
                                {state === 'result' && displayText && (
                                    <motion.div key="result" {...fade}
                                        className="flex items-center gap-2 px-4 w-full"
                                    >
                                        <Check className="w-3.5 h-3.5 shrink-0" style={{ color: '#34d399' }} />
                                        <span className="text-[13px] leading-tight truncate" style={textStyle}>
                                            {displayText}
                                        </span>
                                    </motion.div>
                                )}

                                {/* 错误 */}
                                {state === 'error' && (
                                    <motion.div key="error" {...fade}
                                        className="flex items-center gap-2 px-4 text-xs"
                                        style={{ color: '#fbbf24' }}
                                    >
                                        <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                                        <span>识别失败</span>
                                    </motion.div>
                                )}

                                {/* 断开连接 */}
                                {state === 'disconnected' && (
                                    <motion.div key="disconnected" {...fade}>
                                        <div className="w-2.5 h-2.5 rounded-full bg-red-500/50" />
                                    </motion.div>
                                )}

                                {/* 退出中 */}
                                {state === 'exiting' && (
                                    <motion.div key="exiting" {...fade}
                                        className="text-xs"
                                        style={textSecondaryStyle}
                                    >
                                        再见
                                    </motion.div>
                                )}
                            </div>
                        )}
                    </AnimatePresence>
                </div>
            </motion.div>
        </div>
    );
}
