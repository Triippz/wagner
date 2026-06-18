/* The assistant presence. Same orb full-size on Home and docked in a workspace.
   Drifts as a particle cloud; hue = intent (violet planning, teal executing,
   green done, amber attention); pulses/flickers while "speaking". Honors
   prefers-reduced-motion by drawing one calm static frame. */
export function mountOrb(canvas, opts = {}) {
  const ctx = canvas.getContext('2d');
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const count = opts.count ?? 460;
  const fill = opts.fill ?? 0.30;          // fraction of min(w,h) for radius
  let W, H, cx, cy;
  function size() {
    const r = canvas.getBoundingClientRect();
    W = canvas.width = Math.max(2, r.width * 2);
    H = canvas.height = Math.max(2, r.height * 2);
    cx = W / 2; cy = H / 2;
  }
  size(); addEventListener('resize', size);

  const P = [];
  for (let i = 0; i < count; i++) {
    P.push({ t: Math.acos(2 * Math.random() - 1), p: Math.random() * 6.2832,
             r: 0.72 + Math.random() * 0.28, sp: 0.0006 + Math.random() * 0.0014,
             ph: Math.random() * 6.2832 });
  }

  const state = { hue: 195, target: 195, speaking: false, env: 0.24 };
  const dpr = 2; // canvas is backed at 2x; scale point sizes so they read on HiDPI
  function draw(now) {
    const base = Math.min(W, H) * fill;
    state.hue += (state.target - state.hue) * 0.045;
    const tgt = state.speaking
      ? 0.55 + 0.45 * Math.abs(Math.sin(now * 0.011)) * (0.6 + Math.random() * 0.4)
      : 0.24 + 0.05 * Math.sin(now * 0.0018);
    state.env += (tgt - state.env) * 0.16;
    const pulse = 1 + state.env * 0.16;
    ctx.clearRect(0, 0, W, H);
    // core glow first, so particles sit on top
    const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, base * 1.7);
    g.addColorStop(0, `hsla(${state.hue | 0}, 85%, 60%, ${0.16 + state.env * 0.26})`);
    g.addColorStop(0.5, `hsla(${state.hue | 0}, 85%, 55%, ${0.05 + state.env * 0.12})`);
    g.addColorStop(1, 'transparent');
    ctx.fillStyle = g; ctx.fillRect(0, 0, W, H);
    for (const o of P) {
      o.p += o.sp * 16;
      const wob = Math.sin(now * 0.001 + o.ph) * 0.02;
      const rr = (o.r + wob) * base * pulse;
      const x = Math.sin(o.t) * Math.cos(o.p), y = Math.cos(o.t), z = Math.sin(o.t) * Math.sin(o.p);
      const depth = (z + 1) / 2;
      const a = (0.30 + depth * 0.6) * (0.6 + state.env * 0.8);
      const sz = ((1.0 + depth * 2.6) * pulse + (state.speaking ? Math.random() * depth * 1.4 : 0)) * dpr;
      ctx.beginPath(); ctx.arc(cx + x * rr, cy + y * rr, sz, 0, 6.2832);
      // hsla, not oklch — canvas fillStyle doesn't parse oklch in all engines
      ctx.fillStyle = `hsla(${state.hue | 0}, 88%, ${(58 + depth * 22) | 0}%, ${Math.min(a, 0.98)})`;
      ctx.fill();
    }
  }

  if (reduce) { draw(0); return { set() {}, reduced: true }; }
  let raf;
  (function loop(now) { draw(now); raf = requestAnimationFrame(loop); })(0);
  return {
    set({ hue, speaking } = {}) {
      if (hue != null) state.target = hue;
      if (speaking != null) state.speaking = speaking;
    },
    stop() { cancelAnimationFrame(raf); },
  };
}

/* Intent hue map shared with CSS status colors. */
export const HUE = { planning: 285, executing: 195, done: 150, attention: 85, error: 25 };
