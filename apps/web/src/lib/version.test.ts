import { describe, expect, it } from 'vitest';
import { KERNEXT_VERSION, versionLine } from './version';

describe('version', () => {
  it('injects the workspace version from Cargo.toml at build time', () => {
    expect(KERNEXT_VERSION).toMatch(/^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$/u);
  });

  it('formats the version line like the version built-in', () => {
    expect(versionLine()).toBe(`KERNEXT v${KERNEXT_VERSION}`);
  });
});
