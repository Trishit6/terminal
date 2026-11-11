import { cn } from '@/lib/utils';

type Platform = 'windows' | 'linux' | 'macos';

const MARKS: Record<Platform, { src: string; className: string }> = {
  windows: {
    src: '/brands/windows.svg',
    className: 'platform-mark--windows',
  },
  linux: {
    src: '/brands/linux.svg',
    className: 'platform-mark--linux',
  },
  macos: {
    src: '/brands/apple.svg',
    className: 'platform-mark--apple',
  },
};

export function PlatformMark({
  platform,
  className,
}: {
  platform: Platform;
  className?: string;
}) {
  const mark = MARKS[platform];

  return (
    <img
      className={cn('platform-mark', mark.className, className)}
      src={mark.src}
      alt=""
      aria-hidden="true"
      loading="lazy"
      decoding="async"
    />
  );
}
