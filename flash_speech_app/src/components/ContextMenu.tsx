import { useEffect, useRef } from 'react';
import { Settings, Power } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

interface ContextMenuProps {
    x: number;
    y: number;
    isOpen: boolean;
    onClose: () => void;
    onQuit: () => void;
    onSettings: () => void;
}

export function ContextMenu({ x, y, isOpen, onClose, onQuit, onSettings }: ContextMenuProps) {
    const menuRef = useRef<HTMLDivElement>(null);

    // Close on click outside
    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
                onClose();
            }
        };

        if (isOpen) {
            document.addEventListener('mousedown', handleClickOutside);
        }

        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
        };
    }, [isOpen, onClose]);

    return (
        <AnimatePresence>
            {isOpen && (
                <motion.div
                    ref={menuRef}
                    initial={{ opacity: 0, scale: 0.9 }}
                    animate={{ opacity: 1, scale: 1 }}
                    exit={{ opacity: 0, scale: 0.9 }}
                    transition={{ duration: 0.1 }}
                    className="absolute z-50 min-w-[140px] bg-slate-900/90 backdrop-blur-md border border-white/10 rounded-lg shadow-xl overflow-hidden"
                    style={{ 
                        left: x, 
                        top: y,
                    }}
                >
                    <div className="p-1 space-y-0.5">
                        <button
                            onClick={() => {
                                onSettings();
                                onClose();
                            }}
                            className="w-full flex items-center px-3 py-2 text-sm text-gray-300 hover:text-white hover:bg-white/10 rounded transition-colors"
                        >
                            <Settings className="w-4 h-4 mr-2" />
                            Settings
                        </button>
                        <div className="h-px bg-white/10 my-1 mx-2" />
                        <button
                            onClick={() => {
                                onQuit();
                                onClose();
                            }}
                            className="w-full flex items-center px-3 py-2 text-sm text-rose-400 hover:text-rose-300 hover:bg-white/10 rounded transition-colors"
                        >
                            <Power className="w-4 h-4 mr-2" />
                            Quit
                        </button>
                    </div>
                </motion.div>
            )}
        </AnimatePresence>
    );
}
