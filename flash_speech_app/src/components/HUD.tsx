import { motion, AnimatePresence } from 'framer-motion';
import { Mic, Zap, Check } from 'lucide-react';

interface HUDProps {
    state: 'idle' | 'listening' | 'processing' | 'result';
    text?: string;
    duration?: number;
}

export function HUD({ state, text, duration }: HUDProps) {
    return (
        <div className="flex items-center justify-center">
            <motion.div
                layout
                data-tauri-drag-region
                initial={{ width: 48, height: 48, borderRadius: 24 }}
                animate={{
                    width: state === 'idle' ? 48 : (state === 'result' && text && text.length > 20 ? 'auto' : 320),
                    height: state === 'result' && text && text.length > 50 ? 'auto' : 48,
                    borderRadius: 24
                }}
                transition={{ type: "spring", stiffness: 400, damping: 25 }}
                className="glass-panel relative flex items-center justify-center overflow-hidden"
                style={{ minWidth: 48, minHeight: 48, maxWidth: 600 }}
            >
                {/* Subtle continuous border glow */}
                <div className="absolute inset-0 rounded-full border border-white/10 pointer-events-none" />

                {/* Inner Content */}
                <AnimatePresence mode="wait">

                    {/* IDLE: Breathing Dot */}
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

                    {/* LISTENING: Audio Wave */}
                    {state === 'listening' && (
                        <motion.div
                            key="listening"
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="flex items-center space-x-1 h-4"
                        >
                            <Mic className="w-4 h-4 text-cyan-400 mr-2" />
                            {[...Array(12)].map((_, i) => (
                                <motion.div
                                    key={i}
                                    className="w-0.5 bg-cyan-400 rounded-full"
                                    animate={{
                                        height: [4, 12 + Math.random() * 8, 4],
                                        opacity: [0.3, 1, 0.3]
                                    }}
                                    transition={{
                                        repeat: Infinity,
                                        duration: 0.4,
                                        delay: i * 0.05,
                                        ease: "easeInOut"
                                    }}
                                />
                            ))}
                        </motion.div>
                    )}

                    {/* PROCESSING: Indeterminate Loader */}
                    {state === 'processing' && (
                        <motion.div
                            key="processing"
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="flex items-center space-x-2 text-cyan-500 font-mono text-xs tracking-widest"
                        >
                            <Zap className="w-4 h-4 animate-pulse" />
                            <span>PROCESSING</span>
                        </motion.div>
                    )}

                    {/* RESULT: Clean Text */}
                    {state === 'result' && (
                        <motion.div
                            key="result"
                            initial={{ opacity: 0, scale: 0.95 }}
                            animate={{ opacity: 1, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.95 }}
                            className="px-6 py-3 flex items-center w-full"
                        >
                            <Check className="w-4 h-4 text-emerald-400 mr-3 shrink-0" />
                            <span className="text-white text-sm font-medium leading-relaxed drop-shadow-md">
                                {text}
                            </span>
                        </motion.div>
                    )}

                </AnimatePresence>
            </motion.div>
        </div>
    );
}
