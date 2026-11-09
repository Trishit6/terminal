import { Link } from '@tanstack/react-router';
import { useId } from 'react';
import { cn } from '@/lib/utils';

export function LogoMark({ className }: { className?: string }) {
  const gradient = `kernext-gradient-${useId().replace(/:/g, '')}`;

  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      className={cn('size-9 shrink-0', className)}
    >
      <defs>
        <linearGradient id={gradient} x1="6" y1="6" x2="42" y2="42" gradientUnits="userSpaceOnUse">
          <stop stopColor="#67e8f9" />
          <stop offset="0.5" stopColor="#38bdf8" />
          <stop offset="1" stopColor="#8b5cf6" />
        </linearGradient>
      </defs>
      <rect
        x="1"
        y="1"
        width="46"
        height="46"
        rx="13"
        fill="rgba(5, 7, 13, 0.88)"
        stroke={`url(#${gradient})`}
        strokeWidth="1.4"
      />
      <circle cx="24" cy="24" r="16" stroke={`url(#${gradient})`} strokeWidth="1.1" opacity="0.55" />
      <path
        d="M16.5 12.5v23M16.5 24l8.5-11.5M16.5 24l8.5 11.5"
        stroke={`url(#${gradient})`}
        strokeWidth="3.3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M31.5 12.5v23" stroke="#dff7ff" strokeWidth="2.1" strokeLinecap="round" />
    </svg>
  );
}

export function Logo({
  className,
  markClassName,
}: {
  className?: string;
  markClassName?: string;
}) {
  return (
    <Link
      to="/"
      className={cn(
        'group inline-flex items-center gap-2.5 rounded-lg outline-none',
        className,
      )}
      aria-label="KERNEXT home"
    >
      <LogoMark className={markClassName} />
      <span className="flex flex-col leading-none">
        <span className="font-mono text-[13px] font-semibold tracking-[0.26em] text-foreground">
          KERNEXT
        </span>
        <span className="mt-1 hidden font-mono text-[8px] uppercase tracking-[0.22em] text-muted-foreground sm:block">
          The next core
        </span>
      </span>
    </Link>
  );
}
