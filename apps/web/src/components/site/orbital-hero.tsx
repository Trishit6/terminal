import { Component, useEffect, useRef, useState, type ReactNode } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import gsap from 'gsap';
import * as THREE from 'three';
import { CoreSceneFallback } from '@/components/site/core-fallback';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';

/**
 * The KERNEXT core: one dark metal sphere carrying the K mark, wrapped in three
 * thin elliptical orbital rings with travelling particles.
 *
 * Everything is procedural — textures are drawn to a 2D canvas, the environment
 * map is generated from a canvas, and no asset is fetched at runtime, so the
 * hero renders identically offline.
 */

const SPHERE_RADIUS = 1.15;
/** World-space extent the scene is fitted into (both axes) before scaling. */
const FIT_EXTENT = 4.4;
const CAMERA_DISTANCE = 6.4;
const CAMERA_FOV = 36;

type Vec3 = [number, number, number];

interface RingParticle {
  /** Starting angle along the ring, in radians. */
  offset: number;
  /** Angular speed, radians per second. */
  speed: number;
  /** Particle radius, world units. */
  size: number;
}

interface RingSpec {
  radius: number;
  tube: number;
  /** Euler tilt applied to the ring plane. */
  tilt: Vec3;
  /** Spin of the whole ring around its own axis, radians per second. */
  spin: number;
  /** Angular length of the bright leading arc, radians. */
  arc: number;
  /** Spin of the bright arc, radians per second. */
  arcSpin: number;
  color: string;
  opacity: number;
  particles: RingParticle[];
}

const RINGS: RingSpec[] = [
  {
    radius: 1.58,
    tube: 0.0085,
    tilt: [1.02, 0.12, 0.16],
    spin: 0.1,
    arc: 1.2,
    arcSpin: 0.62,
    color: '#72e8ff',
    opacity: 0.16,
    particles: [
      { offset: 0.2, speed: 0.42, size: 0.027 },
      { offset: 2.3, speed: 0.33, size: 0.018 },
      { offset: 4.4, speed: 0.5, size: 0.013 },
    ],
  },
  {
    radius: 1.92,
    tube: 0.0075,
    tilt: [1.3, 0.38, -0.42],
    spin: -0.07,
    arc: 0.95,
    arcSpin: -0.44,
    color: '#7fb0ff',
    opacity: 0.14,
    particles: [
      { offset: 1.1, speed: 0.3, size: 0.022 },
      { offset: 3.8, speed: 0.24, size: 0.015 },
    ],
  },
  {
    radius: 1.36,
    tube: 0.007,
    tilt: [0.86, -0.5, 0.6],
    spin: 0.15,
    arc: 1.35,
    arcSpin: 0.9,
    color: '#a78bfa',
    opacity: 0.18,
    particles: [
      { offset: 0.7, speed: 0.55, size: 0.02 },
      { offset: 3.1, speed: 0.47, size: 0.013 },
    ],
  },
];

/* ── Procedural textures ─────────────────────────────────────────────────────
   Created once per page load and kept for the lifetime of the document. They
   are shared, immutable, GPU-backed and tiny, so they are intentionally not
   torn down on unmount (which would also break React StrictMode's simulated
   unmount in development). */

let logoTextureCache: THREE.CanvasTexture | null = null;
let glowTextureCache: THREE.CanvasTexture | null = null;

function createLogoTexture(): THREE.CanvasTexture {
  const size = 512;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d');

  if (ctx) {
    const centre = size / 2;

    // Fades out before the sphere silhouette so the emblem reads as a plate
    // sitting on the metal rather than a floating square.
    const plate = ctx.createRadialGradient(centre, centre, 0, centre, centre, centre);
    plate.addColorStop(0, 'rgba(5, 10, 17, 0.94)');
    plate.addColorStop(0.6, 'rgba(5, 10, 17, 0.8)');
    plate.addColorStop(0.82, 'rgba(5, 10, 17, 0.3)');
    plate.addColorStop(0.92, 'rgba(5, 10, 17, 0)');
    plate.addColorStop(1, 'rgba(5, 10, 17, 0)');
    ctx.fillStyle = plate;
    ctx.beginPath();
    ctx.arc(centre, centre, centre, 0, Math.PI * 2);
    ctx.fill();

    // Controlled specular sweep across the top-left of the plate.
    const sheen = ctx.createLinearGradient(0, 0, size, size);
    sheen.addColorStop(0, 'rgba(150, 235, 255, 0.16)');
    sheen.addColorStop(0.45, 'rgba(150, 235, 255, 0)');
    ctx.fillStyle = sheen;
    ctx.beginPath();
    ctx.arc(centre, centre, centre * 0.9, 0, Math.PI * 2);
    ctx.fill();

    // The KERNEXT mark, drawn from the same 48-unit geometry as the SVG logo.
    const unitScale = (size * 0.66) / 32; // logo circle (r=16) spans 66% of canvas
    ctx.save();
    ctx.translate(centre - 24 * unitScale, centre - 24 * unitScale);
    ctx.scale(unitScale, unitScale);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    const mark = ctx.createLinearGradient(6, 6, 42, 42);
    mark.addColorStop(0, '#8beeff');
    mark.addColorStop(0.5, '#38bdf8');
    mark.addColorStop(1, '#8b5cf6');

    ctx.shadowColor = 'rgba(96, 214, 255, 0.75)';
    ctx.shadowBlur = 14;

    ctx.strokeStyle = 'rgba(126, 226, 255, 0.55)';
    ctx.lineWidth = 1.1;
    ctx.beginPath();
    ctx.arc(24, 24, 16, 0, Math.PI * 2);
    ctx.stroke();

    ctx.strokeStyle = mark;
    ctx.lineWidth = 3.3;
    ctx.beginPath();
    ctx.moveTo(16.5, 12.5);
    ctx.lineTo(16.5, 35.5);
    ctx.moveTo(16.5, 24);
    ctx.lineTo(25, 12.5);
    ctx.moveTo(16.5, 24);
    ctx.lineTo(25, 35.5);
    ctx.stroke();

    ctx.shadowBlur = 6;
    ctx.strokeStyle = '#dff7ff';
    ctx.lineWidth = 2.1;
    ctx.beginPath();
    ctx.moveTo(31.5, 12.5);
    ctx.lineTo(31.5, 35.5);
    ctx.stroke();

    ctx.restore();
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = 4;
  return texture;
}

function createGlowTexture(): THREE.CanvasTexture {
  const size = 512;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d');
  const centre = size / 2;

  if (ctx) {
    const glow = ctx.createRadialGradient(centre, centre, 0, centre, centre, centre);
    glow.addColorStop(0, 'rgba(150, 240, 255, 0.62)');
    glow.addColorStop(0.26, 'rgba(64, 158, 255, 0.3)');
    glow.addColorStop(0.58, 'rgba(126, 104, 240, 0.12)');
    glow.addColorStop(1, 'rgba(0, 0, 0, 0)');
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, size, size);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function createEnvironmentTexture(): THREE.CanvasTexture {
  const width = 1024;
  const height = 512;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');

  if (ctx) {
    const sky = ctx.createLinearGradient(0, 0, 0, height);
    sky.addColorStop(0, '#04070c');
    sky.addColorStop(0.46, '#0b1622');
    sky.addColorStop(0.52, '#0f1e2c');
    sky.addColorStop(1, '#05080d');
    ctx.fillStyle = sky;
    ctx.fillRect(0, 0, width, height);

    const blob = (
      x: number,
      y: number,
      radius: number,
      stops: Array<[number, string]>,
    ) => {
      const gradient = ctx.createRadialGradient(x, y, 0, x, y, radius);
      for (const [offset, color] of stops) {
        gradient.addColorStop(offset, color);
      }
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, width, height);
    };

    // Key light, upper left: electric cyan.
    blob(0.2 * width, 0.28 * height, 0.3 * width, [
      [0, 'rgba(150, 240, 255, 0.95)'],
      [0.35, 'rgba(70, 190, 240, 0.42)'],
      [1, 'rgba(0, 0, 0, 0)'],
    ]);
    // Fill light, right: restrained blue.
    blob(0.82 * width, 0.42 * height, 0.26 * width, [
      [0, 'rgba(90, 150, 255, 0.6)'],
      [1, 'rgba(0, 0, 0, 0)'],
    ]);
    // Rim light, lower left: violet.
    blob(0.34 * width, 0.86 * height, 0.24 * width, [
      [0, 'rgba(146, 122, 246, 0.5)'],
      [1, 'rgba(0, 0, 0, 0)'],
    ]);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.mapping = THREE.EquirectangularReflectionMapping;
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function getLogoTexture(): THREE.CanvasTexture | null {
  if (typeof document === 'undefined') {
    return null;
  }
  if (!logoTextureCache) {
    logoTextureCache = createLogoTexture();
  }
  return logoTextureCache;
}

function getGlowTexture(): THREE.CanvasTexture | null {
  if (typeof document === 'undefined') {
    return null;
  }
  if (!glowTextureCache) {
    glowTextureCache = createGlowTexture();
  }
  return glowTextureCache;
}

/* ── Scene ───────────────────────────────────────────────────────────────── */

/**
 * Generates a small PMREM environment map from the procedural canvas so the
 * metal sphere has controlled reflections without fetching an HDR asset.
 */
function SceneEnvironment() {
  const gl = useThree((state) => state.gl);
  const scene = useThree((state) => state.scene);

  useEffect(() => {
    const equirect = createEnvironmentTexture();
    const generator = new THREE.PMREMGenerator(gl);
    const environment = generator.fromEquirectangular(equirect).texture;

    scene.environment = environment;
    scene.environmentIntensity = 1.1;

    equirect.dispose();
    generator.dispose();

    return () => {
      scene.environment = null;
      environment.dispose();
    };
  }, [gl, scene]);

  return null;
}

function Ring({
  spec,
  reducedMotion,
  glowTexture,
}: {
  spec: RingSpec;
  reducedMotion: boolean;
  glowTexture: THREE.CanvasTexture | null;
}) {
  const groupRef = useRef<THREE.Group>(null);
  const arcRef = useRef<THREE.Mesh>(null);
  const particleRefs = useRef<Array<THREE.Group | null>>([]);

  useFrame((state, delta) => {
    if (reducedMotion) {
      return;
    }

    const step = Math.min(delta, 0.05);
    const elapsed = state.clock.elapsedTime;

    if (groupRef.current) {
      groupRef.current.rotation.z += spec.spin * step;
    }
    if (arcRef.current) {
      arcRef.current.rotation.z += spec.arcSpin * step;
    }

    spec.particles.forEach((particle, index) => {
      const node = particleRefs.current[index];
      if (!node) {
        return;
      }
      const angle = particle.offset + particle.speed * elapsed;
      node.position.set(
        Math.cos(angle) * spec.radius,
        Math.sin(angle) * spec.radius,
        0,
      );
    });
  });

  return (
    <group ref={groupRef} rotation={spec.tilt}>
      {/* Faint continuous guide line. */}
      <mesh>
        <torusGeometry args={[spec.radius, spec.tube, 8, 200]} />
        <meshBasicMaterial
          color={spec.color}
          transparent
          opacity={spec.opacity}
          depthWrite={false}
          toneMapped={false}
        />
      </mesh>

      {/* Bright leading arc; its spin is what makes the ring visibly turn. */}
      <mesh ref={arcRef}>
        <torusGeometry args={[spec.radius, spec.tube * 1.9, 8, 160, spec.arc]} />
        <meshBasicMaterial
          color={spec.color}
          transparent
          opacity={0.9}
          depthWrite={false}
          toneMapped={false}
        />
      </mesh>

      {spec.particles.map((particle, index) => (
        <group
          key={index}
          ref={(node) => {
            particleRefs.current[index] = node;
          }}
        >
          <mesh>
            <sphereGeometry args={[particle.size, 12, 12]} />
            <meshBasicMaterial color="#e9feff" toneMapped={false} />
          </mesh>
          {glowTexture ? (
            <sprite scale={particle.size * 9}>
              <spriteMaterial
                map={glowTexture}
                color={spec.color}
                transparent
                depthWrite={false}
                blending={THREE.AdditiveBlending}
                opacity={0.55}
                toneMapped={false}
              />
            </sprite>
          ) : null}
        </group>
      ))}
    </group>
  );
}

function CoreScene({ reducedMotion }: { reducedMotion: boolean }) {
  const fitRef = useRef<THREE.Group>(null);
  const rootRef = useRef<THREE.Group>(null);
  const sphereRef = useRef<THREE.Group>(null);
  const emblemRef = useRef<THREE.Mesh>(null);
  const pointer = useRef({ x: 0, y: 0 });

  const gl = useThree((state) => state.gl);
  const camera = useThree((state) => state.camera);
  const viewport = useThree((state) => state.viewport);

  const logoTexture = getLogoTexture();
  const glowTexture = getGlowTexture();

  // Pointer parallax: listen on the renderer's canvas and clean up on unmount.
  useEffect(() => {
    const element = gl.domElement;

    const onPointerMove = (event: PointerEvent) => {
      const rect = element.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) {
        return;
      }
      pointer.current.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      pointer.current.y = ((event.clientY - rect.top) / rect.height) * 2 - 1;
    };

    const onPointerLeave = () => {
      pointer.current.x = 0;
      pointer.current.y = 0;
    };

    element.addEventListener('pointermove', onPointerMove);
    element.addEventListener('pointerleave', onPointerLeave);

    return () => {
      element.removeEventListener('pointermove', onPointerMove);
      element.removeEventListener('pointerleave', onPointerLeave);
    };
  }, [gl]);

  // Entrance reveal, driven by GSAP. Skipped entirely for reduced motion.
  useEffect(() => {
    if (reducedMotion) {
      return;
    }
    const group = rootRef.current;
    if (!group) {
      return;
    }

    const tween = gsap.fromTo(
      group.scale,
      { x: 0.82, y: 0.82, z: 0.82 },
      { x: 1, y: 1, z: 1, duration: 1.7, ease: 'power3.out' },
    );

    return () => {
      tween.kill();
    };
  }, [reducedMotion]);

  useFrame((_state, delta) => {
    const step = Math.min(delta, 0.05);

    // Keep the whole composition inside the frame at every aspect ratio.
    const fit = Math.min(
      1,
      viewport.width / FIT_EXTENT,
      viewport.height / FIT_EXTENT,
    );

    if (fitRef.current) {
      const scale = THREE.MathUtils.damp(fitRef.current.scale.x, fit, 4, step);
      fitRef.current.scale.setScalar(scale);
    }

    if (rootRef.current) {
      if (reducedMotion) {
        rootRef.current.rotation.x = 0.08;
        rootRef.current.rotation.y = 0.26;
      } else {
        rootRef.current.rotation.x = THREE.MathUtils.damp(
          rootRef.current.rotation.x,
          -pointer.current.y * 0.2,
          2.4,
          step,
        );
        rootRef.current.rotation.y = THREE.MathUtils.damp(
          rootRef.current.rotation.y,
          pointer.current.x * 0.34,
          2.4,
          step,
        );
      }
    }

    if (sphereRef.current && !reducedMotion) {
      sphereRef.current.rotation.y += 0.12 * step;
    }

    // The emblem always faces the viewer so the K mark stays readable and
    // correctly oriented while the sphere turns behind it.
    if (emblemRef.current) {
      emblemRef.current.quaternion.copy(camera.quaternion);
    }
  });

  return (
    <group ref={fitRef}>
      <group ref={rootRef}>
        <SceneEnvironment />

        <ambientLight intensity={0.4} />
        <directionalLight position={[4.5, 6, 5]} intensity={2.1} color="#c6f1ff" />
        <directionalLight position={[-5, -3.5, -4]} intensity={1.05} color="#7c6cf0" />
        <pointLight position={[-3.2, 2.2, 3.4]} intensity={7} distance={14} color="#3fd8ff" />

        {/* Atmospheric glow behind the core. */}
        {glowTexture ? (
          <>
            <sprite position={[0, 0, -1.5]} scale={7.6}>
              <spriteMaterial
                map={glowTexture}
                transparent
                depthWrite={false}
                blending={THREE.AdditiveBlending}
                opacity={0.45}
                toneMapped={false}
              />
            </sprite>
            <sprite position={[0, 0, -0.95]} scale={3.6}>
              <spriteMaterial
                map={glowTexture}
                transparent
                depthWrite={false}
                blending={THREE.AdditiveBlending}
                opacity={0.34}
                toneMapped={false}
              />
            </sprite>
          </>
        ) : null}

        {/* Central metal sphere. */}
        <group ref={sphereRef}>
          <mesh>
            <sphereGeometry args={[SPHERE_RADIUS, 96, 96]} />
            <meshPhysicalMaterial
              color="#0a1017"
              metalness={1}
              roughness={0.3}
              clearcoat={1}
              clearcoatRoughness={0.28}
              envMapIntensity={1.25}
            />
          </mesh>
          {/* Meridian seams make the slow rotation legible on a smooth metal. */}
          <mesh>
            <torusGeometry args={[SPHERE_RADIUS * 1.001, 0.0055, 6, 160]} />
            <meshBasicMaterial color="#04070c" toneMapped={false} />
          </mesh>
          <mesh rotation={[0, Math.PI / 2, 0]}>
            <torusGeometry args={[SPHERE_RADIUS * 1.001, 0.0045, 6, 160]} />
            <meshBasicMaterial color="#0a1622" toneMapped={false} />
          </mesh>
        </group>

        {/* KERNEXT mark, billboarded at the centre of the sphere. */}
        {logoTexture ? (
          <mesh ref={emblemRef} position={[0, 0, SPHERE_RADIUS * 0.9]}>
            <planeGeometry args={[1.34, 1.34]} />
            <meshBasicMaterial
              map={logoTexture}
              transparent
              depthWrite={false}
              toneMapped={false}
            />
          </mesh>
        ) : null}

        {RINGS.map((spec) => (
          <Ring
            key={spec.color}
            spec={spec}
            reducedMotion={reducedMotion}
            glowTexture={glowTexture}
          />
        ))}
      </group>
    </group>
  );
}

/* ── Fallbacks ───────────────────────────────────────────────────────────── */

class SceneErrorBoundary extends Component<
  { fallback: ReactNode; children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

/* ── Public component ────────────────────────────────────────────────────── */

export function OrbitalHero() {
  const reducedMotion = usePrefersReducedMotion();
  const containerRef = useRef<HTMLDivElement>(null);
  const [inView, setInView] = useState(true);
  const [pageVisible, setPageVisible] = useState(true);

  // Stop rendering while the hero is scrolled out of view.
  useEffect(() => {
    const element = containerRef.current;
    if (!element || typeof IntersectionObserver === 'undefined') {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          setInView(entry.isIntersecting);
        }
      },
      { threshold: 0.01 },
    );

    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // Stop rendering while the tab is hidden.
  useEffect(() => {
    const onVisibilityChange = () => {
      setPageVisible(document.visibilityState !== 'hidden');
    };

    onVisibilityChange();
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => document.removeEventListener('visibilitychange', onVisibilityChange);
  }, []);

  const animating = !reducedMotion && inView && pageVisible;

  return (
    <div ref={containerRef} className="orbital-hero" data-reduced-motion={reducedMotion ? 'true' : undefined}>
      <SceneErrorBoundary fallback={<CoreSceneFallback />}>
        <Canvas
          className="orbital-hero__canvas"
          frameloop={animating ? 'always' : 'demand'}
          dpr={[1, 2]}
          camera={{ position: [0, 0, CAMERA_DISTANCE], fov: CAMERA_FOV, near: 0.1, far: 40 }}
          gl={{ antialias: true, alpha: true, powerPreference: 'high-performance' }}
          fallback={<CoreSceneFallback />}
        >
          <CoreScene reducedMotion={reducedMotion} />
        </Canvas>
      </SceneErrorBoundary>
    </div>
  );
}
