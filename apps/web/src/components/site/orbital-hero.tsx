import { Component, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import gsap from 'gsap';
import * as THREE from 'three';
import { CoreSceneFallback } from '@/components/site/core-fallback';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';

/**
 * The KERNEXT core.
 *
 * A single dark metal sphere carrying the K emblem at its centre, wrapped in
 * thin elliptical orbits at different angles and depths. Small cyan, blue and
 * violet particles travel the orbits; a restrained fresnel atmosphere and one
 * soft glow sit behind the sphere.
 *
 * Everything is procedural — the emblem, the atmospheric glow and the
 * environment map are drawn to 2D canvases — so the hero never fetches an
 * asset and renders identically offline. Only pointer parallax is interactive.
 */

const SPHERE_RADIUS = 1.42;
/** World-space extent the whole composition is fitted into, both axes. */
const FIT_EXTENT = 4.9;
const CAMERA_DISTANCE = 6.8;
const CAMERA_FOV = 32;

type Vec3 = [number, number, number];

interface RingParticle {
  /** Starting angle along the orbit, radians. */
  offset: number;
  /** Angular speed, radians per second. */
  speed: number;
  /** Particle radius in world units. */
  size: number;
  /** Pre-computed Z offset for stable 3D depth (set once on mount). */
  zOffset?: number;
}

interface RingSpec {
  radius: number;
  tube: number;
  /** Euler tilt that places the orbit plane in 3D. */
  tilt: Vec3;
  /** Spin of the whole orbit about its own normal, radians/second. */
  spin: number;
  /** Angular length of the bright leading arc, radians. */
  arc: number;
  /** Spin of the bright arc, radians/second. */
  arcSpin: number;
  color: string;
  opacity: number;
  arcOpacity: number;
  particles: RingParticle[];
}

const RINGS: RingSpec[] = [
  {
    radius: 1.62,
    tube: 0.0035,
    tilt: [1.28, 0.12, 0.2],
    spin: 0.05,
    arc: 1.05,
    arcSpin: 0.26,
    color: '#63c2e2',
    opacity: 0.18,
    arcOpacity: 0.35,
    particles: [
      { offset: 0.4, speed: 0.36, size: 0.021, zOffset: 0.12 },
      { offset: 2.9, speed: 0.28, size: 0.014, zOffset: -0.08 },
    ],
  },
  {
    radius: 1.86,
    tube: 0.003,
    tilt: [1.14, 0.48, -0.3],
    spin: -0.04,
    arc: 0.9,
    arcSpin: -0.2,
    color: '#7aa6e6',
    opacity: 0.16,
    arcOpacity: 0.32,
    particles: [
      { offset: 1.3, speed: -0.24, size: 0.017, zOffset: 0.15 },
      { offset: 4.1, speed: 0.2, size: 0.012, zOffset: -0.11 },
    ],
  },
  {
    radius: 2.08,
    tube: 0.0028,
    tilt: [1.44, -0.3, 0.36],
    spin: 0.033,
    arc: 1.2,
    arcSpin: 0.17,
    color: '#9c9be8',
    opacity: 0.15,
    arcOpacity: 0.30,
    particles: [
      { offset: 0.9, speed: 0.32, size: 0.015, zOffset: 0.10 },
      { offset: 3.5, speed: -0.23, size: 0.011, zOffset: -0.09 },
    ],
  },
];

/* ── Procedural textures ────────────────────────────────────────────────────
   Shared, immutable, GPU-backed and small. They are created once per document
   and deliberately kept for its lifetime: tearing them down on unmount would
   also break React StrictMode's simulated unmount in development. */

let logoTextureCache: THREE.CanvasTexture | null = null;
let glowTextureCache: THREE.CanvasTexture | null = null;
let surfaceTextureCache: THREE.CanvasTexture | null = null;

function createLogoTexture(): THREE.CanvasTexture {
  const size = 1024;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d');

  if (ctx) {
    const centre = size / 2;

    // Deliberately no dark backing plate: the sphere's own metal must show
    // through the emblem. A plate here reads as a black disc glued over the
    // core. Only the mark and its hairline ring are painted; everything else
    // stays transparent.

    // The KERNEXT mark, drawn from the same geometry as the SVG logo.
    const unitScale = (size * 0.66) / 32; // logo circle r=16 spans 66%
    ctx.save();
    ctx.translate(centre - 24 * unitScale, centre - 24 * unitScale);
    ctx.scale(unitScale, unitScale);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    // A tight dark halo under the strokes keeps the mark legible when a
    // specular highlight passes behind it, without darkening the whole face.
    ctx.strokeStyle = 'rgba(2, 7, 13, 0.5)';
    ctx.lineWidth = 4.1;
    ctx.beginPath();
    ctx.moveTo(16.5, 12.5);
    ctx.lineTo(16.5, 35.5);
    ctx.moveTo(16.5, 24);
    ctx.lineTo(25, 12.5);
    ctx.moveTo(16.5, 24);
    ctx.lineTo(25, 35.5);
    ctx.moveTo(31.5, 12.5);
    ctx.lineTo(31.5, 35.5);
    ctx.stroke();

    const mark = ctx.createLinearGradient(6, 6, 42, 42);
    mark.addColorStop(0, '#a5f4ff');
    mark.addColorStop(0.48, '#4fd8ea');
    mark.addColorStop(1, '#9b8cff');

    ctx.shadowColor = 'rgba(94, 226, 245, 0.4)';
    ctx.shadowBlur = 10;

    ctx.strokeStyle = 'rgba(132, 226, 245, 0.2)';
    ctx.lineWidth = 0.8;
    ctx.beginPath();
    ctx.arc(24, 24, 16, 0, Math.PI * 2);
    ctx.stroke();

    ctx.strokeStyle = mark;
    ctx.lineWidth = 3.1;
    ctx.beginPath();
    ctx.moveTo(16.5, 12.5);
    ctx.lineTo(16.5, 35.5);
    ctx.moveTo(16.5, 24);
    ctx.lineTo(25, 12.5);
    ctx.moveTo(16.5, 24);
    ctx.lineTo(25, 35.5);
    ctx.stroke();

    ctx.shadowBlur = 0;
    ctx.strokeStyle = '#e6faff';
    ctx.lineWidth = 2;
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
    glow.addColorStop(0, 'rgba(140, 236, 250, 0.5)');
    glow.addColorStop(0.28, 'rgba(58, 142, 232, 0.22)');
    glow.addColorStop(0.62, 'rgba(112, 92, 226, 0.09)');
    glow.addColorStop(1, 'rgba(0, 0, 0, 0)');
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, size, size);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * Faint brushed-metal variation. With a fully metallic material a constant
 * roughness reads as flat black plastic; a little variation gives the sphere
 * readable highlights without adding a texture download.
 */
function createSurfaceTexture(): THREE.CanvasTexture {
  const size = 256;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d');

  if (ctx) {
    // Brushed metal: fine horizontal grain rather than random speckle. A
    // seeded generator keeps the texture identical across reloads, so the
    // orbital fixture can be compared screenshot to screenshot.
    let seed = 0x9e3779b9;
    const random = () => {
      seed = (seed * 1664525 + 1013904223) >>> 0;
      return seed / 4294967296;
    };

    ctx.fillStyle = '#8a8a8a';
    ctx.fillRect(0, 0, size, size);

    for (let y = 0; y < size; y += 1) {
      const shade = 108 + Math.floor(random() * 76);
      ctx.fillStyle = `rgba(${shade}, ${shade}, ${shade}, 0.55)`;
      ctx.fillRect(0, y, size, 1);
    }

    for (let i = 0; i < 160; i += 1) {
      const shade = 156 + Math.floor(random() * 70);
      const y = random() * size;
      const light = random() > 0.5;
      ctx.fillStyle = light
        ? `rgba(${shade}, ${shade}, ${shade}, 0.14)`
        : 'rgba(58, 58, 58, 0.12)';
      ctx.fillRect(random() * size - size * 0.25, y, size * (0.3 + random() * 0.7), 0.8);
    }

    const sweep = ctx.createLinearGradient(0, 0, size, size);
    sweep.addColorStop(0, 'rgba(255, 255, 255, 0.12)');
    sweep.addColorStop(0.42, 'rgba(255, 255, 255, 0)');
    sweep.addColorStop(1, 'rgba(0, 0, 0, 0.16)');
    ctx.fillStyle = sweep;
    ctx.fillRect(0, 0, size, size);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.repeat.set(2, 1);
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
    sky.addColorStop(0, '#050a12');
    sky.addColorStop(0.45, '#0b1624');
    sky.addColorStop(0.54, '#0d1c2c');
    sky.addColorStop(1, '#04070d');
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

    // Key light, upper left: cool cyan.
    blob(0.2 * width, 0.26 * height, 0.29 * width, [
      [0, 'rgba(170, 244, 255, 0.92)'],
      [0.34, 'rgba(64, 180, 226, 0.36)'],
      [1, 'rgba(0, 0, 0, 0)'],
    ]);
    // Fill light, right: restrained blue.
    blob(0.84 * width, 0.44 * height, 0.24 * width, [
      [0, 'rgba(92, 148, 242, 0.5)'],
      [1, 'rgba(0, 0, 0, 0)'],
    ]);
    // Rim light, lower left: violet, deliberately weaker than the key.
    blob(0.32 * width, 0.88 * height, 0.23 * width, [
      [0, 'rgba(142, 120, 246, 0.3)'],
      [1, 'rgba(0, 0, 0, 0)'],
    ]);
    // Broad front softbox. With an equirectangular map, +Z (the camera side)
    // samples at u = 0.75; without light there the core reflects empty sky and
    // reads as a black disc at its centre.
    blob(0.75 * width, 0.48 * height, 0.34 * width, [
      [0, 'rgba(158, 210, 255, 0.55)'],
      [0.34, 'rgba(78, 132, 190, 0.24)'],
      [1, 'rgba(0, 0, 0, 0)'],
    ]);
    // Lower bounce: a faint cool source under the core so the bottom
    // hemisphere keeps its form instead of falling to black.
    blob(0.75 * width, 0.8 * height, 0.26 * width, [
      [0, 'rgba(58, 116, 184, 0.34)'],
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

function getSurfaceTexture(): THREE.CanvasTexture | null {
  if (typeof document === 'undefined') {
    return null;
  }
  if (!surfaceTextureCache) {
    surfaceTextureCache = createSurfaceTexture();
  }
  return surfaceTextureCache;
}

/* ── Atmosphere ──────────────────────────────────────────────────────────── */

const ATMOSPHERE_VERTEX = /* glsl */ `
  varying vec3 vNormal;
  varying vec3 vView;

  void main() {
    vNormal = normalize(normalMatrix * normal);
    vec4 viewPosition = modelViewMatrix * vec4(position, 1.0);
    vView = normalize(-viewPosition.xyz);
    gl_Position = projectionMatrix * viewPosition;
  }
`;

const ATMOSPHERE_FRAGMENT = /* glsl */ `
  varying vec3 vNormal;
  varying vec3 vView;

  void main() {
    float rim = pow(1.0 - abs(dot(normalize(vNormal), normalize(vView))), 2.6);
    vec3 colour = mix(vec3(0.24, 0.86, 0.95), vec3(0.42, 0.56, 0.98), 0.45);
    gl_FragColor = vec4(colour, 1.0) * rim * 0.5;
  }
`;

function Atmosphere() {
  return (
    <mesh scale={SPHERE_RADIUS * 1.035}>
      <sphereGeometry args={[1, 64, 64]} />
      <shaderMaterial
        vertexShader={ATMOSPHERE_VERTEX}
        fragmentShader={ATMOSPHERE_FRAGMENT}
        transparent
        depthWrite={false}
        blending={THREE.AdditiveBlending}
        toneMapped={false}
      />
    </mesh>
  );
}

/**
 * Builds a small PMREM environment from the procedural canvas so the metal
 * sphere has controlled reflections without fetching an HDR asset.
 */
function SceneEnvironment() {
  const gl = useThree((state) => state.gl);
  const scene = useThree((state) => state.scene);

  useEffect(() => {
    const equirect = createEnvironmentTexture();
    const generator = new THREE.PMREMGenerator(gl);
    const environment = generator.fromEquirectangular(equirect).texture;

    scene.environment = environment;
    scene.environmentIntensity = 1.05;

    equirect.dispose();
    generator.dispose();

    return () => {
      scene.environment = null;
      environment.dispose();
    };
  }, [gl, scene]);

  return null;
}

/* ── Orbit ───────────────────────────────────────────────────────────────── */

function Orbit({
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
      // Use pre-computed stable Z offset for smooth 3D depth without jitter.
      const angle = particle.offset + particle.speed * elapsed;
      node.position.set(
        Math.cos(angle) * spec.radius,
        Math.sin(angle) * spec.radius,
        particle.zOffset ?? 0,
      );
    });
  });

  return (
    <group ref={groupRef} rotation={spec.tilt}>
      {/* Faint continuous orbit line. */}
      <mesh>            <torusGeometry args={[spec.radius, spec.tube, 6, 180]} />
        <meshBasicMaterial
          color={spec.color}
          transparent
          opacity={spec.opacity}
          depthWrite={false}
          toneMapped={false}
        />
      </mesh>

      {/* Short leading arc; its slower independent spin makes the orbit turn. */}
      <mesh ref={arcRef}>
        <torusGeometry args={[spec.radius, spec.tube * 1.6, 6, 180, spec.arc]} />
        <meshBasicMaterial
          color={spec.color}
          transparent
          opacity={spec.arcOpacity}
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
            <sphereGeometry args={[particle.size, 10, 10]} />
            <meshBasicMaterial color="#eafdff" toneMapped={false} />
          </mesh>
          {glowTexture ? (
            <sprite scale={particle.size * 8}>
              <spriteMaterial
                map={glowTexture}
                color={spec.color}
                transparent
                depthWrite={false}
                blending={THREE.AdditiveBlending}
                opacity={0.45}
                toneMapped={false}
              />
            </sprite>
          ) : null}
        </group>
      ))}
    </group>
  );
}

/* ── Scene ───────────────────────────────────────────────────────────────── */

function CoreScene({ reducedMotion }: { reducedMotion: boolean }) {
  const fitRef = useRef<THREE.Group>(null);
  const rootRef = useRef<THREE.Group>(null);
  const sphereRef = useRef<THREE.Group>(null);
  const emblemRef = useRef<THREE.Mesh>(null);
  const pointer = useRef({ x: 0, y: 0 });

  const gl = useThree((state) => state.gl);
  const camera = useThree((state) => state.camera);
  const viewport = useThree((state) => state.viewport);

  const logoTexture = useMemo(() => getLogoTexture(), []);
  const glowTexture = useMemo(() => getGlowTexture(), []);
  const surfaceTexture = useMemo(() => getSurfaceTexture(), []);

  // Pointer parallax listens on the renderer's own canvas, so it never
  // captures wheel or touch scrolling.
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

  // Entrance reveal. Skipped entirely when the OS asks for reduced motion.
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
      { x: 0.88, y: 0.88, z: 0.88 },
      { x: 1, y: 1, z: 1, duration: 1.5, ease: 'power3.out' },
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
      // Reduced motion renders a single frame, so it must be fitted exactly
      // rather than eased over frames that will never be drawn.
      const scale = reducedMotion
        ? fit
        : THREE.MathUtils.damp(fitRef.current.scale.x, fit, 4, step);
      fitRef.current.scale.setScalar(scale);
    }

    if (rootRef.current) {
      if (reducedMotion) {
        rootRef.current.rotation.x = 0.06;
        rootRef.current.rotation.y = 0.22;
      } else {
        rootRef.current.rotation.x = THREE.MathUtils.damp(
          rootRef.current.rotation.x,
          -pointer.current.y * 0.14,
          2.2,
          step,
        );
        rootRef.current.rotation.y = THREE.MathUtils.damp(
          rootRef.current.rotation.y,
          pointer.current.x * 0.26,
          2.2,
          step,
        );
      }
    }

    // Very subtle sphere rotation for visual richness while keeping the brand
    // recognizable. Rotation speed scaled down to maintain elegance.
    if (sphereRef.current && !reducedMotion) {
      sphereRef.current.rotation.y += 0.045 * step;
    }

    // Billboard in world space (lookAt accounts for the parallax parent) so
    // the K mark stays upright while the sphere turns behind it.
    if (emblemRef.current) {
      emblemRef.current.lookAt(camera.position);
    }
  });

  return (
    <group ref={fitRef}>
      <group ref={rootRef}>
        <SceneEnvironment />

        <ambientLight intensity={0.22} />
        <hemisphereLight args={['#4a7db4', '#0a1318', 0.45]} />
        <directionalLight position={[4.5, 6, 5]} intensity={1.45} color="#a8dff9" />
        <directionalLight position={[-5, -3.5, -4]} intensity={0.45} color="#7a6ec0" />
        <pointLight position={[-3.2, 2.2, 3.4]} intensity={2.2} distance={14} color="#3fc8e6" />
        {/* Broad camera-side key so the metal face carries a soft gradient
            rather than a black void. Reduced intensity for more balanced studio lighting. */}
        <pointLight position={[0.8, 1.8, 5.4]} intensity={6.5} distance={24} decay={2} color="#b8d8f8" />
        <pointLight position={[-2.4, -3.2, 4.6]} intensity={5.2} distance={18} decay={2} color="#5b8fc8" />
        <directionalLight position={[-4, -5, 3]} intensity={0.52} color="#4a7eb4" />

        {/* Atmospheric glow behind the core. Reduced intensity for more balanced composition. */}
        {glowTexture ? (
          <>
            <sprite position={[0, 0, -1.5]} scale={5.5}>
              <spriteMaterial
                map={glowTexture}
                transparent
                depthWrite={false}
                blending={THREE.AdditiveBlending}
                opacity={0.22}
                toneMapped={false}
              />
            </sprite>
            <sprite position={[0, 0, -0.9]} scale={2.6}>
              <spriteMaterial
                map={glowTexture}
                transparent
                depthWrite={false}
                blending={THREE.AdditiveBlending}
                opacity={0.16}
                toneMapped={false}
              />
            </sprite>
          </>
        ) : null}

        {/* Deep midnight-blue / graphite metallic core with subtle cyan rim reflections.
            Material tuned to avoid excessively bright blue surface while maintaining
            realistic metallic appearance with refined rim lighting. */}
        <group ref={sphereRef} rotation={[0.14, 0, 0.12]}>
          <mesh>
            <sphereGeometry args={[SPHERE_RADIUS, 72, 72]} />
            <meshPhysicalMaterial
              color="#1a2a3a"
              metalness={0.6}
              roughness={0.28}
              roughnessMap={surfaceTexture ?? undefined}
              clearcoat={1}
              clearcoatRoughness={0.15}
              envMapIntensity={1.4}
              ior={1.5}
              thickness={1.2}
            />
          </mesh>
          {/* Subtle cyan rim glow using fresnel-like effect */}
          <mesh rotation={[0, 0, 0]}>
            <sphereGeometry args={[SPHERE_RADIUS, 64, 64]} />
            <meshBasicMaterial
              color="#4fd8ea"
              transparent
              opacity={0.08}
              side={THREE.BackSide}
              blending={THREE.AdditiveBlending}
              depthWrite={false}
              toneMapped={false}
            />
          </mesh>
          {/* Two very faint seams make the slow rotation legible on smooth metal. */}
          <mesh>
            <torusGeometry args={[SPHERE_RADIUS * 1.001, 0.0025, 6, 180]} />
            <meshBasicMaterial color="#0f1820" toneMapped={false} />
          </mesh>
          <mesh rotation={[0, Math.PI / 2, 0]}>
            <torusGeometry args={[SPHERE_RADIUS * 1.001, 0.002, 6, 180]} />
            <meshBasicMaterial color="#161d26" toneMapped={false} />
          </mesh>
        </group>

        <Atmosphere />

        {/* KERNEXT emblem. Reduced in size by applying a 75% scale to create a more
            balanced composition (1.62 * 0.75 = 1.215). Centered perfectly, undistorted.
            Positioned just in front of the sphere's surface (z = radius) so the metal
            never occludes the mark, and billboarded so the K stays upright while the
            core turns. Draws last and ignores depth so no foreground orbit can cut it;
            its transparent pixels still show the metal. */}
        {logoTexture ? (
          <mesh ref={emblemRef} position={[0, 0, SPHERE_RADIUS]} renderOrder={20}>
            <planeGeometry args={[1.215, 1.215]} />
            <meshBasicMaterial
              map={logoTexture}
              transparent
              depthWrite={false}
              depthTest={false}
              toneMapped={false}
            />
          </mesh>
        ) : null}

        {RINGS.map((spec) => (
          <Orbit
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

export function OrbitalHero({ forceReducedMotion = false }: { forceReducedMotion?: boolean }) {
  const prefersReducedMotion = usePrefersReducedMotion();
  const reducedMotion = prefersReducedMotion || forceReducedMotion;
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
    <div
      ref={containerRef}
      className="orbital-hero"
      data-reduced-motion={reducedMotion ? 'true' : undefined}
    >
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
