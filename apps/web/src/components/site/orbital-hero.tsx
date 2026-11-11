import { motion } from 'motion/react';
import { Cpu, Globe2, Network, Waves, Zap } from 'lucide-react';
import { LogoMark } from '@/components/site/logo';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';

/**
 * The KERNEXT orbital hero.
 *
 * Three independent layers:
 *
 * 1. A CSS perspective stage that gives the rings real depth without rotating
 *    the terminal preview or any surrounding text.
 * 2. Two counter-rotating orbital planes, each on its own speed and tilt.
 * 3. A stationary central sphere and emblem. Only the emblem floats (and
 *    slightly responds to hover), so the mark stays recognizable.
 *
 * Labels live outside the 3D planes, so they never rotate or blur. Reduced
 * Motion stops the orbits and floating while keeping the full composition
 * readable.
 */
export function OrbitalHero() {
  const reduced = usePrefersReducedMotion();

  return (
    <div
      className="relative mx-auto aspect-square w-full min-w-0 max-w-[440px] select-none"
      style={{ perspective: '1200px' }}
    >
      <div
        className="absolute inset-0"
        style={{ transformStyle: 'preserve-3d' }}
        aria-hidden="true"
      >
        {/* Ambient light behind the sphere, kept restrained. */}
        <div className="absolute inset-[14%] rounded-full bg-primary/12 blur-[70px]" />

        {/* Outer orbital plane: slow clockwise rotation, cyan. */}
        <div
          className="absolute inset-[2%]"
          style={{ transform: 'rotateX(64deg)', transformStyle: 'preserve-3d' }}
        >
          <div className="absolute inset-0 rounded-full border border-primary/25 shadow-[0_0_50px_rgba(34,211,238,0.12)]" />
          <motion.div
            data-orbital-ring="outer"
            className="absolute inset-0"
            animate={reduced ? undefined : { rotate: 360 }}
            transition={{ duration: 34, ease: 'linear', repeat: Infinity }}
            style={{ willChange: 'transform' }}
          >
            <div className="absolute inset-0 rounded-full border-t-2 border-primary/60 opacity-80" />
            <span className="orbital-point absolute left-1/2 top-0 size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-primary shadow-[0_0_16px_5px_rgba(34,211,238,0.55)]" />
          </motion.div>
        </div>

        {/* Inner orbital plane: opposite direction, slightly faster, violet. */}
        <div
          className="absolute inset-[19%]"
          style={{
            transform: 'rotateX(74deg) rotateZ(-16deg)',
            transformStyle: 'preserve-3d',
          }}
        >
          <div className="absolute inset-0 rounded-full border border-accent/30 shadow-[0_0_36px_rgba(139,92,246,0.14)]" />
          <motion.div
            data-orbital-ring="inner"
            className="absolute inset-0"
            animate={reduced ? undefined : { rotate: -360 }}
            transition={{ duration: 22, ease: 'linear', repeat: Infinity }}
            style={{ willChange: 'transform' }}
          >
            <div className="absolute inset-0 rounded-full border-b-2 border-accent/60 opacity-70" />
            <span className="absolute bottom-0 left-1/2 size-1.5 -translate-x-1/2 translate-y-1/2 rounded-full bg-accent shadow-[0_0_12px_4px_rgba(167,139,250,0.45)]" />
          </motion.div>
        </div>

        {/* Central sphere: layered gradients rather than a single glow. */}
        <div className="absolute inset-0 grid place-items-center">
          <div
            className="relative grid size-[42%] place-items-center rounded-full border border-white/8"
            style={{
              background:
                'radial-gradient(circle at 34% 26%, rgba(56,84,120,0.55) 0%, rgba(12,22,40,0.96) 42%, rgba(4,8,16,1) 70%), radial-gradient(circle at 70% 78%, rgba(34,211,238,0.12), transparent 55%)',
              boxShadow:
                'inset 0 1px 0 rgba(255,255,255,0.08), inset -18px -22px 46px rgba(2,6,16,0.9), 0 24px 60px -24px rgba(2,8,23,0.95)',
            }}
          >
            <div className="pointer-events-none absolute inset-0 rounded-full bg-[radial-gradient(circle_at_64%_18%,rgba(255,255,255,0.10),transparent_42%)]" />
            <motion.div
              animate={reduced ? undefined : { y: [0, -5, 0] }}
              transition={{ duration: 7, ease: 'easeInOut', repeat: Infinity }}
              whileHover={reduced ? undefined : { scale: 1.045, rotateY: 7 }}
              className="relative z-10"
              style={{ transformStyle: 'preserve-3d' }}
            >
              <LogoMark className="size-16 drop-shadow-[0_10px_24px_rgba(34,211,238,0.25)] sm:size-20" />
            </motion.div>
          </div>
        </div>
      </div>

      <TechLabel
        className="left-0 top-[13%] lg:-left-3"
        icon={Cpu}
        label="Rust engine"
      />
      <TechLabel
        className="right-0 top-[8%] lg:-right-2"
        icon={Globe2}
        label="Cross-platform"
      />
      <TechLabel
        className="right-0 top-[40%] lg:-right-4"
        icon={Waves}
        label="ConPTY"
      />
      <TechLabel
        className="bottom-[26%] right-[2%] lg:-right-2"
        icon={Zap}
        label="Native execution"
      />
      <TechLabel
        className="bottom-[18%] left-[2%] lg:-left-2"
        icon={Network}
        label="WebSocket"
      />
    </div>
  );
}

function TechLabel({
  className,
  icon: Icon,
  label,
}: {
  className?: string;
  icon: typeof Cpu;
  label: string;
}) {
  const reduced = usePrefersReducedMotion();

  return (
    <motion.div
      animate={reduced ? undefined : { y: [0, -4, 0] }}
      transition={{
        duration: 6.5,
        ease: 'easeInOut',
        repeat: Infinity,
        delay: label.length % 3,
      }}
      className={`absolute z-20 hidden items-center gap-1.5 rounded-full border border-border-strong bg-surface/90 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground shadow-[0_10px_30px_-18px_rgba(2,8,23,0.9)] sm:flex ${className ?? ''}`}
    >
      <Icon className="size-3 text-primary" aria-hidden="true" />
      {label}
    </motion.div>
  );
}
