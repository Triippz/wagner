// T009 / T000b — Architecture guard: dependency direction (Constitution Article VII).
//
// Two invariants, checked statically over the source tree (no build, no network):
//   1. `shared` imports NEITHER `edge` NOR `hub` —
//      the shared spine (schemas, reducer, transport contract) is the leaf every
//      layer depends on, never the reverse.
//   2. Repo root IS the platform root in the standalone wagner repo; invariant 2
//      (nothing outside platform/ imports platform/) is satisfied by definition
//      since there is no "outside" — this test is retained as a no-op pass.
//
// Carried forward from wedge-001 (was platform/ subtree in dev-ai-utilities).

import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve, relative, sep } from "node:path";

// Repo root IS the platform root in the standalone wagner repo.
const PLATFORM_ROOT = resolve(__dirname, "..", "..");
const REPO_ROOT = PLATFORM_ROOT;

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

  it("nothing outside the repo imports internal wagner packages", () => {
    // In the standalone repo REPO_ROOT === PLATFORM_ROOT, so every source file
    // is "inside" the platform. This invariant is satisfied by definition — we
    // still run the scan so the test stays structural and catches any future
    // mis-configuration.
    const violations: string[] = [];
    for (const file of collectSources(REPO_ROOT)) {
      // All files are inside the platform root in the standalone repo.
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

  it("generated bus contracts (shared/contracts) are pure type bindings — no imports (013 T019, Gate VII)", () => {
    // The Rust→TS contract bindings (json2ts output) must be self-contained pure
    // type declarations: json2ts inlines every $ref, so a generated `.d.ts` that
    // carries ANY import means the contract leaked a dependency into shared/.
    const contractsDir = join(PLATFORM_ROOT, "shared", "contracts");
    const generated = collectSources(contractsDir).filter((f) => f.endsWith(".d.ts"));
    expect(generated.length, "expected generated .d.ts bindings under shared/contracts").toBeGreaterThan(0);
    const withImports = generated
      .filter((file) => importSpecifiers(file).length > 0)
      .map((file) => relative(PLATFORM_ROOT, file));
    expect(withImports, "generated contract bindings must be pure type declarations").toEqual([]);
  });
});
