# Wagner Hub — multi-stage Docker build (Deno + Hono)
#
# Stage 1 (deps): cache the module graph so rebuilds after source-only changes
# don't re-fetch the internet. Stage 2 (runtime): copy source + cached modules,
# run as a non-root user.
#
# Build: docker build -t wagner-hub hub/
# Run:   docker run -p 8787:8787 \
#          -e OIDC_ALLOWED_ISSUERS=https://accounts.google.com \
#          -e OIDC_CLIENT_ID=<client-id> \
#          -e OIDC_ALLOWED_DOMAINS=yourcompany.io \
#          wagner-hub

# ── Stage 1: dependency cache ────────────────────────────────────────────────
FROM denoland/deno:2.3.5 AS deps

# The denoland/deno base image stores its module cache at /deno-dir (DENO_DIR).
WORKDIR /app/hub

# Copy only the files that drive module resolution so layer cache is effective.
COPY deno.json ./
COPY src/ ./src/

# Cache the full module graph (downloads jsr:@hono/hono, npm:jose, etc.).
# --no-lock skips lock-file creation inside the build; the deno.json pins versions.
RUN deno cache src/main.ts

# ── Stage 2: runtime ─────────────────────────────────────────────────────────
FROM denoland/deno:2.3.5

WORKDIR /app/hub

# Copy the populated Deno module cache from the deps stage (/deno-dir).
COPY --from=deps /deno-dir /deno-dir

# Copy application source.
COPY --from=deps /app/hub/deno.json ./
COPY --from=deps /app/hub/src/ ./src/

# Create and switch to a non-root user (uid 1001) so the container does not
# run as root in production.
RUN addgroup --gid 1001 --system hubgroup && \
    adduser --uid 1001 --system --ingroup hubgroup --no-create-home hubuser && \
    chown -R hubuser:hubgroup /app/hub

USER hubuser

EXPOSE 8787

# Healthcheck hits the unauthenticated /health probe (defined in app.ts).
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
  CMD deno eval "const r = await fetch('http://localhost:8787/health'); if (!r.ok) Deno.exit(1)"

# Config comes entirely from the environment (D-SEC-2; no secrets baked in).
# --allow-read is needed for Deno to load source files from the working dir.
CMD ["deno", "run", "--allow-net", "--allow-env", "--allow-read", \
     "--no-prompt", "src/main.ts"]
