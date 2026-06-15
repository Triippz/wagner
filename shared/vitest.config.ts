import { defineConfig } from "vitest/config";

// The shared spine is pure (no DOM, no I/O) — Article VIII. Node environment.
export default defineConfig({
  test: {
    environment: "node",
    include: ["**/*.test.ts"],
  },
});
