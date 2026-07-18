// Shared, framework-free helpers for the FIELD mark layer. The WebGL renderer
// uploads sizes/flags computed here and reproduces the colour ramp in-shader;
// the 2D fallback and the SVG layers call these directly. One source of truth.

// Riverbase stance axis: coral (against) → pale → emerald (pro), per the mock.
// Stance stays colour-blind safe because shape is redundant (down-chevron
// oppose / bar neutral / up-chevron support) — colour is never the only channel.
export const OPPOSE = [245, 144, 120]
export const NEUTRAL = [201, 214, 229]
export const SUPPORT = [31, 176, 163]

const lerp = (a, b, t) => a + (b - a) * t

// Diverging coral→pale→emerald ramp, mirrored in the fragment shader.
export function stanceRGB(s) {
  const t = Math.max(-1, Math.min(1, s))
  return t < 0
    ? [0, 1, 2].map((i) => lerp(NEUTRAL[i], OPPOSE[i], -t))
    : [0, 1, 2].map((i) => lerp(NEUTRAL[i], SUPPORT[i], t))
}
export const stanceCss = (s) => { const [r, g, b] = stanceRGB(s); return `rgb(${r | 0}, ${g | 0}, ${b | 0})` }

// Radius grows with how much the graph actually knows. Demo marks carry no
// score and keep the fixed size. (mirror of #5's markSize.)
export const markRadius = (m) =>
  m.evidence_score != null ? Math.max(3.5, Math.min(9, 3.5 + Math.sqrt(m.evidence_score) * 1.3)) : 5

export const isRefused = (m) => m.eligible === false
export const isSynthetic = (m) => !!m.synthetic

// The recolour sweep (redesign-plan §3.6): 700ms, ease-out-quint, spatially
// staggered outward from the originating Persona.
export const SWEEP_MS = 700
export const STAGGER_MS = 360
export const easeOutQuint = (t) => 1 - Math.pow(1 - t, 5)

export const scalarClass = (s) => (s < -0.15 ? 'oppose' : s > 0.15 ? 'support' : 'neutral')
export const scalarLabel = (s) => (s < -0.15 ? 'Opposes' : s > 0.15 ? 'Supports' : 'Neutral')
