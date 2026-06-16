// T009 / T000b — Architecture guard: dependency direction (Constitution Article VII).
//
// Two invariants, checked statically over the source tree (no build, no network):
//   1. `platform/shared` imports NEITHER `platform/edge` NOR `platform/hub` —
//      the shared spine (schemas, reducer, transport contract) is the leaf every
//      layer depends on, never the reverse.
//   2. Nothing OUTSIDE `platform/` imports `platform/` — the wedge is self-contained
//      (apps/, scripts/, plugins/, etc. must not reach into it).
//
// Carried forward from wedge-001 and required green by wedge-002 (T009).

import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve, relative, sep } from "node:path";

const PLATFORM_ROOT = resolve(__dirname, "..", "..");
const REPO_ROOT = resolve(PLATFORM_ROOT, "..");

const SOURCE_EXT = /\.(ts|tsx|js|jsx|mjs|cjs)$/;
const SKIP_DIRS = new Set([
  "node_modules",
  "target",
  ".git",
  "dist",
  "build",
  ".backups",
  "coverage",
]);

/** Recursively collect source files under `dir`, skipping vendored/build dirs. */
function collectSources(dir: string): string[] {
  const out: string[] = [];
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
  }
  for (const name of entries) {
    if (SKIP_DIRS.has(name)) continue;
    const full = join(dir, name);
    let s;
    try {
      s = statSync(full);
    } catch {
      continue;
    }
    if (s.isDirectory()) out.push(...collectSources(full));
    else if (SOURCE_EXT.test(name)) out.push(full);
  }
  return out;
}

/** Extract the specifier of every static/dynamic import and re-export. */
function importSpecifiers(src: string): string[] {
  const specs: string[] = [];
  const patterns = [
    /\bimport\s+[^'"]*?from\s*['"]([^'"]+)['"]/g, // import x from '...'
    /\bimport\s*['"]([^'"]+)['"]/g, // bare import '...'
    /\bexport\s+[^'"]*?from\s*['"]([^'"]+)['"]/g, // re-export from '...'
    /\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)/g, // dynamic import('...')
    /\brequire\s*\(\s*['"]([^'"]+)['"]\s*\)/g, // require('...')
  ];
  for (const re of patterns) {
    let m: RegExpExecArray | null;
    while ((m = re.exec(src)) !== null) specs.push(m[1]);
  }
  return specs;
}

/** Resolve a relative specifier against the importing file's dir; null otherwise. */
function resolveImport(fromFile: string, spec: string): string | null {
  if (spec.startsWith(".")) return resolve(join(fromFile, ".."), spec);
  return null; // bare/package specifier — handled by name-based rules
}

describe("Article VII — dependency direction", () => {
  it("platform/shared imports neither platform/edge nor platform/hub", () => {
    const sharedDir = join(PLATFORM_ROOT, "shared");
    const edgeDir = join(PLATFORM_ROOT, "edge");
    const hubDir = join(PLATFORM_ROOT, "hub");
    const violations: string[] = [];

    for (const file of collectSources(sharedDir)) {
      for (const spec of importSpecifiers(file)) {
        // Name-based: workspace aliases into edge/hub.
        if (/@wagner\/(edge|hub)/.test(spec) || /(^|\/)(edge|hub)\//.test(spec)) {
          const resolved = resolveImport(file, spec);
          // For relative specs, only flag if they actually land in edge/ or hub/.
          if (spec.startsWith(".")) {
            if (resolved && (resolved.startsWith(edgeDir) || resolved.startsWith(hubDir))) {
              violations.push(`${relative(PLATFORM_ROOT, file)} -> ${spec}`);
            }
          } else {
            violations.push(`${relative(PLATFORM_ROOT, file)} -> ${spec}`);
          }
        }
      }
    }

    expect(violations, `shared/ must not depend on edge/ or hub/:\n${violations.join("\n")}`).toEqual([]);
  });

  it("nothing outside platform/ imports platform/", () => {
    const violations: string[] = [];
    for (const file of collectSources(REPO_ROOT)) {
      // Only inspect files that live OUTSIDE platform/.
      const rel = relative(PLATFORM_ROOT, file);
      const isInsidePlatform = !rel.startsWith("..") && !rel.startsWith(sep) && rel !== "";
      if (isInsidePlatform) continue;

      for (const spec of importSpecifiers(file)) {
        if (/@wagner\/(platform|shared|edge|hub|arch-tests)\b/.test(spec)) {
          violations.push(`${relative(REPO_ROOT, file)} -> ${spec}`);
          continue;
        }
        if (spec.startsWith(".")) {
          const resolved = resolveImport(file, spec);
          if (resolved && resolved.startsWith(PLATFORM_ROOT + sep)) {
            violations.push(`${relative(REPO_ROOT, file)} -> ${spec}`);
          }
        }
      }
    }

    expect(violations, `external code must not import platform/:\n${violations.join("\n")}`).toEqual([]);
  });
});
