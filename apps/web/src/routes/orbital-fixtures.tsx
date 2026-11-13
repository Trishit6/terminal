import { useState } from 'react';
import { OrbitalHero } from '@/components/site/orbital-hero';
import { Button } from '@/components/ui/button';

/**
 * Orbital hero rendering fixtures.
 *
 * A backend-free page that renders the real `OrbitalHero` scene in isolation.
 * It exists to catch the emblem regressions that are hard to see in production
 * copy: a dark backing plate painted behind the K, an orbit crossing the mark,
 * a missing ring, or a core that reads as a flat black disc.
 *
 * Motion is frozen by default so a screenshot is deterministic. The backdrop
 * can be switched to a light studio or a checker so a dark plate and the
 * sphere silhouette are immediately visible.
 */

const SIZES = [
  { id: 'compact', label: '24rem', width: '24rem' },
  { id: 'medium', label: '32rem', width: '32rem' },
  { id: 'full', label: '41rem', width: '41rem' },
] as const;

type SizeId = (typeof SIZES)[number]['id'];
type Backdrop = 'studio' | 'checker' | 'light';

const BACKDROPS: Array<{ id: Backdrop; label: string }> = [
  { id: 'studio', label: 'Studio' },
  { id: 'checker', label: 'Checker' },
  { id: 'light', label: 'Light' },
];

export function OrbitalFixturesPage() {
  const [size, setSize] = useState<SizeId>('full');
  const [backdrop, setBackdrop] = useState<Backdrop>('studio');
  const [animated, setAnimated] = useState(false);

  const width = SIZES.find((entry) => entry.id === size)?.width ?? '41rem';

  return (
    <div className="mx-auto flex w-full max-w-site flex-col px-4 py-8 sm:px-6 lg:px-8">
      <div className="max-w-2xl">
        <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-primary">
          Rendering fixtures
        </p>
        <h1 className="mt-1.5 text-3xl font-semibold tracking-[-0.03em] sm:text-4xl">
          Orbital hero render test
        </h1>
        <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
          The real <code className="rounded bg-surface-soft px-1 font-mono text-[11px]">OrbitalHero</code>{' '}
          scene without homepage copy. Check that the complete KERNEXT K sits on
          exposed metal, exactly three rings orbit it, no ring crosses the mark,
          and the core has a lit edge and readable metal gradient rather than a
          black disc glued over its centre.
        </p>
      </div>

      <div className="mt-5 flex flex-wrap items-center gap-2">
        {SIZES.map((entry) => (
          <Button
            key={entry.id}
            size="sm"
            variant={size === entry.id ? 'default' : 'secondary'}
            onClick={() => setSize(entry.id)}
            aria-pressed={size === entry.id}
          >
            {entry.label}
          </Button>
        ))}
        <span className="mx-1 h-5 w-px bg-border" aria-hidden="true" />
        {BACKDROPS.map((entry) => (
          <Button
            key={entry.id}
            size="sm"
            variant={backdrop === entry.id ? 'default' : 'outline'}
            onClick={() => setBackdrop(entry.id)}
            aria-pressed={backdrop === entry.id}
          >
            {entry.label}
          </Button>
        ))}
        <span className="mx-1 h-5 w-px bg-border" aria-hidden="true" />
        <Button
          size="sm"
          variant={animated ? 'default' : 'outline'}
          onClick={() => setAnimated((current) => !current)}
          aria-pressed={animated}
        >
          {animated ? 'Motion on' : 'Motion frozen'}
        </Button>
      </div>

      <div
        className={[
          'relative mt-5 flex w-full justify-center overflow-hidden rounded-xl border border-border px-4 py-8 sm:py-12',
          backdrop === 'studio' ? 'bg-background' : '',
          backdrop === 'checker' ? 'orbital-fixture__checker' : '',
          backdrop === 'light' ? 'bg-[#d6e0ea]' : '',
        ].join(' ')}
        data-orbital-fixture={backdrop}
      >
        <div
          className="relative aspect-square w-full min-w-0 max-w-full"
          style={{ width }}
          data-orbital-size={size}
        >
          <OrbitalHero forceReducedMotion={!animated} />
        </div>
      </div>

      <div className="mt-4 grid gap-1 text-xs leading-relaxed text-muted-foreground">
        <p>
          <b className="font-medium text-foreground">Must never regress:</b> a dark
          radial plate or square behind the K, an opaque halo wider than the
          strokes, a fourth orbit, or a ring drawn across the emblem.
        </p>
        <p>
          The checker and light backdrops expose a painted plate immediately;
          keep the K fully inside the sphere and the three rings clear of it at
          every size.
        </p>
      </div>
    </div>
  );
}
