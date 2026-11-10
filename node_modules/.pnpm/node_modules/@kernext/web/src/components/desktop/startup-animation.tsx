import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'motion/react';

const DURATION_MS = 1500;

/**
 * The full cinematic desktop startup presentation.
 *
 * The CLI keeps its text-only emblem; the desktop shell can use the richer
 * variant: a metallic K monogram, an orbital ring, a mountain silhouette, and
 * a glowing planetary horizon.
 */
export function DesktopStartupAnimation({ visible }: { visible: boolean }) {
  const [rendered, setRendered] = useState(visible);

  useEffect(() => {
    if (visible) {
      setRendered(true);
      const timer = window.setTimeout(() => setRendered(false), DURATION_MS);
      return () => window.clearTimeout(timer);
    }

    setRendered(false);
    return undefined;
  }, [visible]);

  return (
    <AnimatePresence>
      {rendered ? (
        <motion.div
          key="startup"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.5, ease: 'easeInOut' }}
          className="fixed inset-0 z-50 overflow-hidden bg-[#04070c]"
          aria-hidden="true"
        >
          <div className="absolute inset-0 aurora opacity-70" />

          <svg
            className="absolute inset-x-0 bottom-0 h-[42%] w-full"
            viewBox="0 0 1440 420"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            <defs>
              <linearGradient id="mountain-fill" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="#0b1220" />
                <stop offset="100%" stopColor="#04070c" />
              </linearGradient>
              <linearGradient id="ridge-fill" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="#111c2e" />
                <stop offset="100%" stopColor="#050810" />
              </linearGradient>
            </defs>

            <path
              d="M0 420 L0 250 L180 190 L300 260 L430 130 L560 240 L700 150 L840 250 L980 170 L1120 260 L1260 180 L1440 250 L1440 420 Z"
              fill="url(#mountain-fill)"
            />
            <path
              d="M0 420 L0 300 L160 250 L320 300 L470 220 L620 300 L780 240 L940 300 L1100 240 L1260 300 L1440 260 L1440 420 Z"
              fill="url(#ridge-fill)"
              opacity="0.85"
            />
          </svg>

          <div className="absolute left-1/2 top-[38%] h-[360px] w-[360px] -translate-x-1/2 -translate-y-1/2">
            <div className="absolute inset-0 rounded-full border border-primary/20 bg-primary/5 blur-[80px]" />
            <div className="orbital absolute inset-0">
              <motion.div
                initial={{ opacity: 0, scale: 0.8 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.7, ease: 'easeOut' }}
                className="grid size-full place-items-center"
              >
                <span className="bg-gradient-to-br from-[#e0f7ff] via-[#67e8f9] to-[#818cf8] bg-clip-text text-[190px] font-black leading-none tracking-[-0.08em] text-transparent">
                  K
                </span>
              </motion.div>
              <span className="orbital-dot" />
            </div>
          </div>

          <div className="absolute left-1/2 top-[62%] h-[280px] w-[520px] -translate-x-1/2 rounded-full bg-[radial-gradient(closest-side,rgba(34,211,238,0.18),transparent)] blur-3xl" />

          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.35, ease: 'easeOut' }}
            className="absolute inset-x-0 bottom-[16%] flex flex-col items-center gap-2"
          >
            <span className="font-mono text-3xl font-semibold uppercase tracking-[0.42em] text-foreground/90">
              kernext
            </span>
            <span className="font-mono text-[10px] uppercase tracking-[0.32em] text-muted-foreground">
              the next core of computing
            </span>
          </motion.div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
