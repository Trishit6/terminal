import { useRef, useEffect, useState } from 'react';
import { Canvas } from '@react-three/fiber';
import { usePrefersReducedMotion } from '@/hooks/use-prefers-reduced-motion';

export function OrbitalHero() {
  const reducedMotion = usePrefersReducedMotion();
  const isDark = false;
  const sceneRef = useRef<HTMLDivElement>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    setReady(true);
  }, []);

  const heroStyle = {
    position: 'relative',
    width: '100%',
    height: 'calc(60vh + 200px)',
    minHeight: '400px',
    overflow: 'hidden',
    background: isDark ? '#0a0a0f' : '#0d0d1a',
  } as React.CSSProperties;

  const canvasStyle = {
    position: 'absolute',
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
  } as React.CSSProperties;

  return (
    <div
      ref={sceneRef}
      className="orbital-hero"
      data-reduced-motion={reducedMotion ? 'true' : undefined}
      style={heroStyle}
    >
      <Canvas
        style={canvasStyle}
        gl={{ antialias: true, alpha: false }}
        onCreated={() => setReady(true)}
      >
        <ambientLight intensity={0.6} color={isDark ? '#88c0d0' : '#4a90d9'} />
        <directionalLight
          position={[5, 10, 8]}
          intensity={0.8}
          color={isDark ? '#6dd5fa' : '#2c5aa0'}
          castShadow
        />
        <directionalLight
          position={[-5, -10, -8]}
          intensity={0.5}
          color={isDark ? '#4a4a6a' : '#5a7ab0'}
        />

        {/* Central KERNEXT sphere */}
        <mesh position={[-0.5, -0.2, 0]} receiveShadow castShadow>
          <sphereGeometry args={[1, 64, 64]} />
          <meshStandardMaterial
            metalness={0.7}
            roughness={0.3}
            flatShading={false}
            color={isDark ? '#1a1a2e' : '#0d0d1a'}
          />
        </mesh>

        {/* Core logo at center */}
        <mesh position={[0, 0, 0]}> 
          <sphereGeometry args={[0.3, 32, 32]} /> 
          <meshStandardMaterial
            metalness={0.9}
            roughness={0.1}
            color={isDark ? '#0f0f23' : '#080810'}
          />
        </mesh>

        {/* Orbital ring 1 - slow, wide, cyan */}
        <mesh rotation={[-Math.PI / 2, 0, 0]} scale={1.3}>
          <torusGeometry args={[1.5, 0.15, 32, 64]} />
          <meshStandardMaterial
            color={isDark ? '#00b8d9' : '#00d4ff'}
            metalness={0.4}
            roughness={0.6}
            transparent
          />
        </mesh>

        {/* Orbital ring 2 - medium, blue */}
        <mesh rotation={[0, Math.PI / 4, 0]} scale={1.5}>
          <torusGeometry args={[1.2, 0.12, 32, 64]} />
          <meshStandardMaterial
            color={isDark ? '#3b82f6' : '#3b99fc'}
            metalness={0.5}
            roughness={0.5}
            transparent
          />
        </mesh>

        {/* Orbital ring 3 - tight, violet */}
        <mesh rotation={[Math.PI / 3, Math.PI / 6, 0]} scale={1.1}>
          <torusGeometry args={[0.9, 0.1, 32, 64]} />
          <meshStandardMaterial
            color={isDark ? '#8b5cf6' : '#a78bfa'}
            metalness={0.6}
            roughness={0.4}
            transparent
          />
        </mesh>

        {/* Glowing particles along orbits */}
        <group>
          {Array.from({ length: 15 }).map((__, i) => {
            const angle = (i / 15) * Math.PI * 2;
            const radius = 1.2 + Math.random() * 0.4;
            return (
              <mesh
                key={i}
                position={[Math.cos(angle) * radius, Math.sin(angle) * 0.5, 0]}
              >
                <sphereGeometry args={[0.08, 8, 8]} />
                <meshStandardMaterial
                  color={isDark ? '#00b8d9' : '#00d4ff'}
                  emissive={isDark ? '#006b8e' : '#0096c7'}
                  emissiveIntensity={0.5}
                />
              </mesh>
            );
          })}
        </group>
      </Canvas>

      {reducedMotion && !ready && (
        <div
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: '100%',
            background: '#0a0a0f',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#334155',
            fontFamily: 'var(--font-sans)',
          }}
        >
          <span>KERNEXT — Independent. Native. Engineered.</span>
        </div>
      )}

      {!reducedMotion && !ready && (
        <div
          style={{
            position: 'absolute',
            bottom: 20,
            left: '50%',
            transform: 'translateX(-50%)',
            color: isDark ? '#94a3b8' : '#64748b',
            fontFamily: 'var(--font-sans)',
            fontSize: '0.875rem',
          }}
        >
          KERNEXT engine core
        </div>
      )}
    </div>
  );
}
