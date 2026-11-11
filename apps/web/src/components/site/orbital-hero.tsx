import { motion } from 'motion/react';
import { Cpu, Globe2, Network, Waves, Zap } from 'lucide-react';
import { LogoMark } from '@/components/site/logo';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';

export function OrbitalHero() {
  const reduced = usePrefersReducedMotion();

  return (
    <div
      className="orbital-stage relative mx-auto aspect-square w-full min-w-0 max-w-[500px] select-none"
      style={{ perspective: '1800px' }}
    >
      <div className="absolute inset-[6%] rounded-[38%] border border-primary/20 opacity-70 blur-[1px]" />
      <div className="absolute inset-[10%] rounded-[32%] border border-accent/20 opacity-70 blur-[1px]" />
      <div className="hero-particles absolute inset-0" aria-hidden="true" />

      <motion.div
        animate={reduced ? undefined : { rotateY: [0, 8, 0], rotateX: [0, 10, 0] }}
        transition={{ duration: 18, ease: 'easeInOut', repeat: Infinity }}
        style={{ transformStyle: 'preserve-3d' }}
        className="absolute inset-0"
      >
        <div className="absolute inset-[4%] rounded-full border border-primary/25 shadow-[0_0_80px_rgba(34,211,238,0.08)]" style={{ transform: 'rotateX(68deg) translateZ(12px)' }} />
        <motion.div
          data-orbital-ring="outer"
          className="absolute inset-[2%]"
          animate={reduced ? undefined : { rotate: 360 }}
          transition={{ duration: 34, ease: 'linear', repeat: Infinity }}
          style={{ transform: 'rotateX(68deg) rotateZ(16deg)', transformStyle: 'preserve-3d', willChange: 'transform' }}
        >
          <div className="absolute inset-0 rounded-full border border-primary/30 shadow-[0_0_60px_rgba(34,211,238,0.22)]" />
          <div className="absolute inset-[9%] rounded-full border border-primary/25" />
          <div className="absolute inset-[22%] rounded-full border border-primary/15" />
          <span className="absolute left-1/2 top-[6%] size-3 -translate-x-1/2 rounded-full bg-primary shadow-[0_0_18px_rgba(34,211,238,0.7)]" />
          <span className="absolute bottom-[8%] left-[20%] size-2.5 rounded-full bg-cyan-300/80 shadow-[0_0_18px_rgba(103,232,249,0.7)]" />
        </motion.div>

        <div className="absolute inset-[18%] rounded-full border border-accent/30 shadow-[0_0_40px_rgba(139,92,246,0.1)]" style={{ transform: 'rotateX(76deg) rotateY(18deg) rotateZ(-12deg)', transformStyle: 'preserve-3d' }} />
        <motion.div
          data-orbital-ring="inner"
          className="absolute inset-[18%]"
          animate={reduced ? undefined : { rotate: -360 }}
          transition={{ duration: 22, ease: 'linear', repeat: Infinity }}
          style={{ transform: 'rotateX(76deg) rotateY(18deg) rotateZ(-12deg)', transformStyle: 'preserve-3d', willChange: 'transform' }}
        >
          <div className="absolute inset-0 rounded-full border border-accent/45" />
          <div className="absolute inset-[12%] rounded-full border border-accent/30" />
          <span className="absolute bottom-4 left-1/2 size-2.5 -translate-x-1/2 rounded-full bg-accent shadow-[0_0_18px_rgba(168,85,247,0.7)]" />
          <span className="absolute left-[12%] top-1/2 size-2 rounded-full bg-violet-300/80 shadow-[0_0_16px_rgba(196,181,253,0.7)]" />
        </motion.div>

        <div className="absolute inset-0 grid place-items-center">
          <motion.div
            animate={reduced ? undefined : { y: [0, -8, 0], rotateZ: [0, 1.5, 0] }}
            transition={{ duration: 7, ease: 'easeInOut', repeat: Infinity }}
            whileHover={reduced ? undefined : { scale: 1.08, rotateY: 10 }}
            className="relative grid place-items-center"
            style={{ transformStyle: 'preserve-3d' }}
          >
            <div className="absolute inset-[-16%] rounded-full bg-[radial-gradient(circle,rgba(34,211,238,0.20),rgba(34,211,238,0.05)_26%,transparent_60%)] blur-2xl" />
            <div
              className="relative grid size-[190px] place-items-center rounded-full border border-white/10 bg-[radial-gradient(circle_at_35%_30%,rgba(96,165,250,0.18),rgba(3,7,18,0.85)_38%,rgba(2,6,23,1)_74%)] shadow-[inset_0_1px_0_rgba(255,255,255,0.08),inset_-12px_-20px_34px_rgba(2,6,16,0.9),0_32px_90px_-30px_rgba(8,145,178,0.7)] sm:size-[220px]"
              style={{ transform: 'translateZ(44px)' }}
            >
              <div className="absolute inset-[8%] rounded-full border border-white/10" />
              <div className="absolute inset-[18%] rounded-full border border-sky-400/20" />
              <LogoMark className="relative z-10 size-16 drop-shadow-[0_12px_28px_rgba(34,211,238,0.28)] sm:size-20" />
            </div>
          </motion.div>
        </div>
      </motion.div>

      <TechLabel className="left-0 top-[16%] lg:-left-4" icon={Cpu} label="Rust engine" />
      <TechLabel className="right-0 top-[14%] lg:-right-3" icon={Globe2} label="Cross-platform" />
      <TechLabel className="right-1 top-[42%] lg:right-1" icon={Waves} label="ConPTY" />
      <TechLabel className="bottom-[22%] right-[2%] lg:-right-2" icon={Zap} label="Native execution" />
      <TechLabel className="bottom-[18%] left-[2%] lg:-left-3" icon={Network} label="WebSocket" />
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
      transition={{ duration: 6.5, ease: 'easeInOut', repeat: Infinity, delay: label.length % 3 }}
      className={`tech-chip absolute z-20 hidden items-center gap-1.5 sm:flex ${className ?? ''}`}
    >
      <Icon className="size-3 text-primary" aria-hidden="true" />
      <span>{label}</span>
    </motion.div>
  );
}
