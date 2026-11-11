import { useRef, type PointerEvent } from 'react';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';
import { LogoMark } from '@/components/site/logo';

export function OrbitalHero() {
  const sceneRef = useRef<HTMLDivElement>(null);
  const reducedMotion = usePrefersReducedMotion();

  function handlePointerMove(event: PointerEvent<HTMLDivElement>) {
    if (reducedMotion || event.pointerType !== 'mouse') {
      return;
    }

    const bounds = event.currentTarget.getBoundingClientRect();
    if (bounds.width === 0 || bounds.height === 0) {
      return;
    }

    const x = ((event.clientX - bounds.left) / bounds.width - 0.5) * 2;
    const y = ((event.clientY - bounds.top) / bounds.height - 0.5) * 2;

    sceneRef.current?.style.setProperty('--pointer-x', `${(x * 7).toFixed(2)}px`);
    sceneRef.current?.style.setProperty('--pointer-y', `${(y * 6).toFixed(2)}px`);
    sceneRef.current?.style.setProperty('--pointer-rotate-x', `${(y * -2).toFixed(2)}deg`);
    sceneRef.current?.style.setProperty('--pointer-rotate-y', `${(x * 3).toFixed(2)}deg`);
    sceneRef.current?.style.setProperty('--core-light-x', `${(34 + x * 8).toFixed(2)}%`);
    sceneRef.current?.style.setProperty('--core-light-y', `${(22 + y * 7).toFixed(2)}%`);
  }

  function resetPointer() {
    sceneRef.current?.style.setProperty('--pointer-x', '0px');
    sceneRef.current?.style.setProperty('--pointer-y', '0px');
    sceneRef.current?.style.setProperty('--pointer-rotate-x', '0deg');
    sceneRef.current?.style.setProperty('--pointer-rotate-y', '0deg');
    sceneRef.current?.style.setProperty('--core-light-x', '34%');
    sceneRef.current?.style.setProperty('--core-light-y', '22%');
  }

  return (
    <div
      ref={sceneRef}
      className="core-scene"
      data-reduced-motion={reducedMotion ? 'true' : undefined}
      onPointerMove={handlePointerMove}
      onPointerLeave={resetPointer}
    >
      <span className="sr-only">
        KERNEXT emblem on a slowly turning metallic computing core.
      </span>
      <div className="core-scene__depth" aria-hidden="true">
        <div className="core-orbit core-orbit--primary">
          <div className="core-orbit__track">
            <span className="core-orbit__satellite" />
          </div>
        </div>
        <div className="core-orbit core-orbit--secondary">
          <div className="core-orbit__track">
            <span className="core-orbit__satellite" />
          </div>
        </div>
        <div className="core-object-float">
          <div className="core-object">
            <div className="core-object__back" />
            <div className="core-object__face">
              <span className="core-object__mark">
                <LogoMark className="size-20 sm:size-24" />
              </span>
            </div>
            <div className="core-object__rear">
              <span className="core-object__mark">
                <LogoMark className="size-20 sm:size-24" />
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
