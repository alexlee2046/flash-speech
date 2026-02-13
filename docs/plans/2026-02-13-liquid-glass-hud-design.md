# FlashSpeech Liquid Glass HUD Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform the FlashSpeech HUD from dark glassmorphism to Apple-style Liquid Glass with morphing animations.

**Architecture:** CSS-only liquid glass material (backdrop-filter + multi-layer gradients + box-shadow) with Framer Motion spring animations for morphing state transitions. All decorative layers (highlight rotation, rim light, ripple) are pure CSS to keep JS thread free. No SVG filters (unsupported in WKWebView).

**Tech Stack:** React 19, Framer Motion 11, Tailwind 3, CSS @keyframes, Tauri v1 (WKWebView/Safari)

---

## Context

- **Project root:** `flash_speech/flash_speech_app/`
- **Dev server:** `npm run dev` (Vite on port 1421)
- **Full app:** `npm run tauri dev` (requires Rust toolchain)
- **Build target:** Safari 13 (WKWebView on macOS)
- **No test framework** — verification is visual via dev server
- **HUD window:** transparent, always-on-top, no decorations, 44px height pill

## File Overview

| File | Role | Change |
|------|------|--------|
| `src/index.css` | Global styles + glass material | **Rewrite**: replace `.glass-pill` with `.liquid-glass` + animation keyframes |
| `src/components/HUD.tsx` | HUD component (all states) | **Rewrite**: new colors, sizes, structure with decorative layers |
| `src/App.tsx` | Root component | No change |
| `src/main.tsx` | Entry point | No change |
| `tailwind.config.ts` | Tailwind config | No change |

---

### Task 1: Rewrite CSS — Liquid Glass Material & Animations

**Files:**
- Modify: `src/index.css` (full rewrite of lines 22-31, add new classes)

**Step 1: Replace `.glass-pill` with `.liquid-glass` material**

Replace the entire `src/index.css` with:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

html, body, #root {
  background: transparent !important;
  margin: 0;
  padding: 0;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif;
  -webkit-font-smoothing: antialiased;
}

#root {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ── Liquid Glass Material ── */
.liquid-glass {
  position: relative;
  background: rgba(255, 255, 255, 0.35);
  backdrop-filter: blur(40px) saturate(180%) brightness(105%);
  -webkit-backdrop-filter: blur(40px) saturate(180%) brightness(105%);
  border: 0.5px solid rgba(255, 255, 255, 0.4);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.5),
    inset 0 -0.5px 0 rgba(0, 0, 0, 0.05),
    0 0.5px 1px rgba(0, 0, 0, 0.08),
    0 4px 16px rgba(0, 0, 0, 0.06),
    0 8px 32px rgba(0, 0, 0, 0.04);
  overflow: hidden;
}

/* ── Highlight Layer: rotating light sweep ── */
.liquid-glass-highlight {
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background: linear-gradient(
    var(--highlight-angle, 135deg),
    rgba(255, 255, 255, 0.55) 0%,
    rgba(255, 255, 255, 0.08) 50%,
    transparent 100%
  );
  pointer-events: none;
  animation: highlight-rotate 8s linear infinite;
}

@keyframes highlight-rotate {
  from { --highlight-angle: 135deg; }
  to   { --highlight-angle: 495deg; }
}

/* Register custom property for animation (WebKit) */
@property --highlight-angle {
  syntax: "<angle>";
  inherits: false;
  initial-value: 135deg;
}

/* ── Rim Light: subtle rainbow border ── */
.liquid-glass-rim {
  position: absolute;
  inset: -1px;
  border-radius: inherit;
  background: conic-gradient(
    from var(--rim-angle, 0deg),
    rgba(255, 120, 120, 0.4),
    rgba(255, 200, 100, 0.4),
    rgba(100, 255, 150, 0.4),
    rgba(100, 180, 255, 0.4),
    rgba(200, 130, 255, 0.4),
    rgba(255, 120, 120, 0.4)
  );
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  padding: 1px;
  pointer-events: none;
  animation: rim-rotate 6s linear infinite;
  opacity: 0.7;
}

@keyframes rim-rotate {
  from { --rim-angle: 0deg; }
  to   { --rim-angle: 360deg; }
}

@property --rim-angle {
  syntax: "<angle>";
  inherits: false;
  initial-value: 0deg;
}

/* Listening state: rim turns red */
.liquid-glass-rim.listening {
  background: conic-gradient(
    from var(--rim-angle, 0deg),
    rgba(255, 80, 80, 0.6),
    rgba(255, 120, 100, 0.5),
    rgba(255, 80, 80, 0.6),
    rgba(255, 60, 60, 0.5),
    rgba(255, 80, 80, 0.6)
  );
  opacity: 0.9;
  animation: rim-rotate 3s linear infinite;
}

/* ── Ripple effect (triggered via JS adding .ripple-active) ── */
.liquid-glass-ripple {
  position: absolute;
  width: 60px;
  height: 60px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(255,255,255,0.6) 0%, transparent 70%);
  pointer-events: none;
  transform: translate(-50%, -50%) scale(0);
  opacity: 0;
}

.liquid-glass-ripple.active {
  animation: ripple-expand 300ms ease-out forwards;
}

@keyframes ripple-expand {
  0%   { transform: translate(-50%, -50%) scale(0); opacity: 0.8; }
  100% { transform: translate(-50%, -50%) scale(2.5); opacity: 0; }
}

/* ── State flash: brief brightness boost on transitions ── */
.liquid-glass-flash {
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background: rgba(255, 255, 255, 0);
  pointer-events: none;
  transition: background 0.15s ease-out;
}

.liquid-glass-flash.active {
  background: rgba(255, 255, 255, 0.15);
}
```

**Step 2: Verify CSS compiles**

Run: `cd /Users/alex/Develop/daoime/flash_speech/flash_speech_app && npx vite build 2>&1 | head -20`
Expected: Build succeeds (CSS is valid)

**Step 3: Commit**

```bash
git add src/index.css
git commit -m "style: replace glass-pill with liquid-glass CSS material and animations"
```

---

### Task 2: Rewrite HUD.tsx — Structure, Sizes & Colors

**Files:**
- Modify: `src/components/HUD.tsx` (full rewrite)

**Step 1: Write the complete new HUD component**

Replace `src/components/HUD.tsx` with:

```tsx
import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { appWindow, LogicalSize, LogicalPosition, currentMonitor } from '@tauri-apps/api/window';
import { motion, AnimatePresence } from 'framer-motion';
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
        // Trigger ripple at click position
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
                <div className="liquid-glass-highlight" />

                {/* Rim light layer */}
                <div className={`liquid-glass-rim ${state === 'listening' ? 'listening' : ''}`} />

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
```

Key changes from original:
- `PILL_H`: 48 → 44 (slightly more compact)
- `spring`: stiffness 500→400, damping 32→28 (softer, more fluid feel)
- Class: `glass-pill` → `liquid-glass`
- All colors: white/light → dark on light background (see design doc §4)
- Added decorative layers: `.liquid-glass-highlight`, `.liquid-glass-rim`, `.liquid-glass-flash`, `.liquid-glass-ripple`
- Added `flash` state: brief brightness pulse on every state transition
- Added `ripple` state: radial light burst on right-click
- Rim light gains `.listening` class during recording (red tint)
- All content elements get `relative z-10` to render above decorative layers
- Pill sizes adjusted per design doc §2

**Step 2: Verify TypeScript compiles**

Run: `cd /Users/alex/Develop/daoime/flash_speech/flash_speech_app && npx tsc --noEmit 2>&1 | head -20`
Expected: No errors (or only pre-existing ones from Tauri types)

**Step 3: Verify Vite build**

Run: `cd /Users/alex/Develop/daoime/flash_speech/flash_speech_app && npx vite build 2>&1 | tail -10`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add src/components/HUD.tsx
git commit -m "feat: rewrite HUD with liquid glass material, morphing animations, and light effects"
```

---

### Task 3: Visual Verification & Polish

**Step 1: Start dev server and visually verify**

Run: `cd /Users/alex/Develop/daoime/flash_speech/flash_speech_app && npm run dev`

Open `http://localhost:1421` in Safari (to match WKWebView behavior). Check:
- [ ] Liquid glass material renders (bright translucent background with blur)
- [ ] Highlight gradient rotates smoothly
- [ ] Rainbow rim light is visible and animates
- [ ] Text is legible on the bright background

**Step 2: Test `@property` fallback for Safari**

`@property` (CSS Houdini) is needed for animating `--highlight-angle` and `--rim-angle`. Safari 15.4+ supports it. The build target is Safari 13.

If `@property` doesn't work in WKWebView, replace the CSS keyframe approach with a JS-driven angle update using Framer Motion's `useMotionValue` + `useTransform`:

```tsx
// Fallback: add to HUD component if @property fails
const highlightAngle = useMotionValue(135);
useEffect(() => {
    const controls = animate(highlightAngle, 495, {
        duration: 8,
        repeat: Infinity,
        ease: "linear",
    });
    return controls.stop;
}, []);
```

Then apply via inline style: `style={{ background: useTransform(highlightAngle, v => `linear-gradient(${v}deg, ...)`) }}`.

**Step 3: Adjust if needed and commit polish**

```bash
git add -A
git commit -m "style: polish liquid glass visual tuning"
```

---

## Verification Checklist

After all tasks:

1. `npx tsc --noEmit` — no type errors
2. `npx vite build` — build succeeds
3. Visual in Safari:
   - Liquid glass material visible (translucent white, blurred)
   - Highlight rotates
   - Rim light animates (rainbow normally, red during listening)
   - Flash fires on state change
   - Ripple fires on right-click
   - All text legible (dark on light)
   - Pill morphs fluidly between states

## References

- [Apple Liquid Glass 解析 (CSS-Tricks)](https://css-tricks.com/getting-clarity-on-apples-liquid-glass/)
- [CSS/SVG Liquid Glass 实现 (LogRocket)](https://blog.logrocket.com/how-create-liquid-glass-effects-css-and-svg/)
- [CSS Liquid Glass 效果合集](https://freefrontend.com/css-liquid-glass/)
