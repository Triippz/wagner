/* The knowledge graph as the centerpiece — a dense white wireframe plexus
   (nodes = notes/learnings, edges = links) slowly rotating in 3D. Status lives in
   tight color clusters: lime = verified/active, amber = needs review, red =
   disputed. It also IS the assistant's presence: pulses/brightens when Wagner
   speaks. prefers-reduced-motion → one static frame.
   Canvas fills use hsla (oklch isn't parsed by all canvas engines). */
export function mountGraph(canvas, opts = {}) {
  const ctx = canvas.getContext('2d');
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const N = opts.count ?? 210;
  const fill = opts.fill ?? 0.40;
  const K = opts.links ?? 4;            // edges per node — denser mesh
  let W, H, cx, cy, R;
  function size() {
    const r = canvas.getBoundingClientRect();
    W = canvas.width = Math.max(2, r.width * 2);
    H = canvas.height = Math.max(2, r.height * 2);
    cx = W / 2; cy = H / 2; R = Math.min(W, H) * fill;
  }
  size(); addEventListener('resize', size);

  let s = 9301; const rnd = () => ((s = (s * 9301 + 49297) % 233280) / 233280);
  const norm = (x, y, z) => { const m = Math.hypot(x, y, z) || 1; return [x/m, y/m, z/m]; };
  const LIME = norm(0.85, -0.35, 0.25);   // verified/active cluster direction
  const AMBER = norm(-0.75, 0.45, -0.2);  // needs-review cluster direction

  // cohesive ovoid volume: shell-weighted but filled; tight angular clusters
  const nodes = [];
  for (let i = 0; i < N; i++) {
    const t = Math.acos(2 * rnd() - 1), p = rnd() * 6.2832;
    const rad = 0.42 + 0.58 * Math.sqrt(rnd());
    const x = Math.sin(t) * Math.cos(p) * rad;
    const y = Math.cos(t) * rad * 1.18;
    const z = Math.sin(t) * Math.sin(p) * rad;
    const [ux, uy, uz] = norm(x, y, z);
    let g = 'base';
    if (ux*LIME[0] + uy*LIME[1] + uz*LIME[2] > 0.82) g = 'lime';
    else if (ux*AMBER[0] + uy*AMBER[1] + uz*AMBER[2] > 0.84) g = 'warn';
    else if (rnd() > 0.99) g = 'alert';
    nodes.push({ x, y, z, g });
  }
  // edges: each node to its K nearest (the fine triangulated mesh)
  const edges = [];
  for (let i = 0; i < N; i++) {
    const d = [];
    for (let j = 0; j < N; j++) if (j !== i) {
      const a = nodes[i], b = nodes[j];
      d.push([(a.x-b.x)**2 + (a.y-b.y)**2 + (a.z-b.z)**2, j]);
    }
    d.sort((m, n) => m[0] - n[0]);
    for (let k = 0; k < K; k++) { const j = d[k][1]; if (i < j) edges.push([i, j]); }
  }

  const COL = {
    base:  (a) => `hsla(140,4%,90%,${a})`,
    lime:  (a) => `hsla(96,82%,62%,${a})`,
    warn:  (a) => `hsla(56,90%,60%,${a})`,
    alert: (a) => `hsla(9,80%,58%,${a})`,
  };
  const state = { speaking: false, env: 0.2, a: 0 };

  function draw() {
    state.a += reduce ? 0 : 0.0013;
    const tgt = state.speaking ? 0.62 + 0.38 * Math.abs(Math.sin(state.a * 11)) : 0.2;
    state.env += (tgt - state.env) * (reduce ? 1 : 0.1);
    const pulse = 1 + state.env * 0.05;
    const cos = Math.cos(state.a), sin = Math.sin(state.a), fov = R * 2.8;
    const proj = (n) => {
      const rx = n.x * cos - n.z * sin, rz = n.x * sin + n.z * cos;
      const sc = fov / (fov - rz * R) * pulse;
      return { sx: cx + rx * R * sc, sy: cy + n.y * R * sc, depth: (rz + 1) / 2 };
    };
    ctx.clearRect(0, 0, W, H);
    const P = nodes.map(proj);
    ctx.lineWidth = 1;
    for (const [i, j] of edges) {
      const a = P[i], b = P[j], depth = (a.depth + b.depth) / 2;
      const g = nodes[i].g !== 'base' ? nodes[i].g : nodes[j].g;
      const al = (0.035 + depth * 0.11) * (0.65 + state.env * 0.6);
      ctx.strokeStyle = g === 'base' ? `hsla(140,4%,82%,${al})` : COL[g](al * 1.5);
      ctx.beginPath(); ctx.moveTo(a.sx, a.sy); ctx.lineTo(b.sx, b.sy); ctx.stroke();
    }
    for (let i = 0; i < N; i++) {
      const a = P[i], n = nodes[i];
      const al = (0.4 + a.depth * 0.55) * (0.65 + state.env * 0.55);
      const sz = (0.7 + a.depth * 1.7) * (n.g === 'base' ? 1 : 1.6) * 2;
      if (n.g !== 'base') { // glow under status nodes
        ctx.beginPath(); ctx.arc(a.sx, a.sy, sz * 3.2, 0, 6.2832);
        ctx.fillStyle = COL[n.g](0.08 + state.env * 0.1); ctx.fill();
      }
      ctx.beginPath(); ctx.arc(a.sx, a.sy, sz, 0, 6.2832);
      ctx.fillStyle = COL[n.g](Math.min(al, 0.98)); ctx.fill();
    }
  }

  if (reduce) { draw(); return { set() {}, reduced: true }; }
  let raf; (function loop() { draw(); raf = requestAnimationFrame(loop); })();
  return {
    set({ speaking } = {}) { if (speaking != null) state.speaking = speaking; },
    stop() { cancelAnimationFrame(raf); },
  };
}
