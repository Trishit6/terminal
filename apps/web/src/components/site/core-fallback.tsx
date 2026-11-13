import { LogoMark } from '@/components/site/logo';

/**
 * Static, dependency-free stand-in for the Three.js core scene.
 *
 * It is rendered while the 3D chunk is still loading (Suspense fallback), when
 * the browser cannot create a WebGL context, and if the scene throws at
 * runtime. Keeping it free of `three`/`@react-three/fiber` imports means the
 * marketing copy paints immediately and the ~1 MB 3D bundle stays off the
 * critical path.
 */
export function CoreSceneFallback() {
  return (
    <div className="orbital-fallback" role="img" aria-label="KERNEXT engine core">
      <span className="orbital-fallback__glow" aria-hidden="true" />
      <span className="orbital-fallback__ring orbital-fallback__ring--a" aria-hidden="true" />
      <span className="orbital-fallback__ring orbital-fallback__ring--b" aria-hidden="true" />
      <LogoMark className="size-16" />
      <span className="orbital-fallback__caption">KERNEXT core</span>
    </div>
  );
}
