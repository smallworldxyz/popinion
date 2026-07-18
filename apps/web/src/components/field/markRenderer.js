// WebGL2 point-sprite renderer for the FIELD mark layer (redesign-plan §10).
//
// - one static position VBO uploaded once (positions are frozen per basemap)
// - per-round Float32Arrays of from/to stance + stagger delay, the recolour
//   sweep is eased in the vertex shader so 1000+ marks never touch the CPU
//   per frame and never touch Vue reactivity
// - chevron / bar / hollow-ring glyphs from a 2x2 sprite atlas, the cell chosen
//   from the interpolated stance + flags
// - picking via an offscreen ID-colour buffer (gl_VertexID, WebGL2)
//
// Falls back to a plain 2D renderer (same API) when WebGL2 is unavailable, so
// the map is never blank on a weak WebKitGTK driver.

import { stanceRGB, markRadius, isRefused, isSynthetic, easeOutQuint } from './marks'

const DRAW_VS = `#version 300 es
precision highp float;
layout(location=0) in vec2 a_pos;    // frozen, [0,1]
layout(location=1) in float a_from;  // stance at sweep start
layout(location=2) in float a_to;    // stance target this round
layout(location=3) in float a_delay; // stagger, seconds
layout(location=4) in float a_size;  // radius in css px (0 => refused sentinel handled by flags)
layout(location=5) in float a_flags; // 0 normal, 1 refused, 2 synthetic
uniform vec2 u_res;    // canvas px
uniform float u_pad;   // px
uniform float u_time;  // seconds since round change
uniform float u_dur;   // sweep seconds
uniform float u_dpr;
uniform int u_selected;
out float v_stance;
out float v_alpha;
flat out int v_cell;
void main() {
  float W = u_res.x, H = u_res.y;
  float x = u_pad + a_pos.x * (W - 2.0 * u_pad);
  float y = (H - u_pad) - a_pos.y * (H - 2.0 * u_pad);
  gl_Position = vec4(x / W * 2.0 - 1.0, -(y / H * 2.0 - 1.0), 0.0, 1.0);

  float t = clamp((u_time - a_delay) / u_dur, 0.0, 1.0);
  float e = 1.0 - pow(1.0 - t, 5.0);
  float stance = mix(a_from, a_to, e);
  v_stance = stance;

  bool refused = abs(a_flags - 1.0) < 0.5;
  bool synthetic = abs(a_flags - 2.0) < 0.5;
  bool sel = (gl_VertexID == u_selected);
  int cell = refused ? 3 : (stance < -0.15 ? 0 : (stance > 0.15 ? 1 : 2));
  v_cell = cell;

  float radius = refused ? (sel ? 5.0 : 3.5) : (a_size + (sel ? 2.0 : 0.0));
  gl_PointSize = (radius * 2.0 + 4.0) * u_dpr;
  v_alpha = refused ? (sel ? 0.7 : 0.45) : (synthetic ? 0.5 : 1.0);
}`

const DRAW_FS = `#version 300 es
precision highp float;
uniform sampler2D u_atlas;
in float v_stance;
in float v_alpha;
flat in int v_cell;
out vec4 frag;
vec3 stanceColor(float s) {
  vec3 O = vec3(70.0, 120.0, 224.0) / 255.0;
  vec3 N = vec3(176.0, 178.0, 190.0) / 255.0;
  vec3 S = vec3(212.0, 158.0, 60.0) / 255.0;
  float t = clamp(s, -1.0, 1.0);
  return t < 0.0 ? mix(N, O, -t) : mix(N, S, t);
}
void main() {
  vec2 origin = vec2((v_cell == 1 || v_cell == 3) ? 0.5 : 0.0, v_cell >= 2 ? 0.5 : 0.0);
  float a = texture(u_atlas, origin + gl_PointCoord * 0.5).a;
  if (a < 0.02) discard;
  vec3 col = v_cell == 3 ? vec3(0.5, 0.5, 0.52) : stanceColor(v_stance);
  frag = vec4(col, a * v_alpha);
}`

const PICK_VS = `#version 300 es
precision highp float;
layout(location=0) in vec2 a_pos;
layout(location=4) in float a_size;
layout(location=5) in float a_flags;
uniform vec2 u_res;
uniform float u_pad;
uniform float u_dpr;
flat out vec3 v_id;
void main() {
  float W = u_res.x, H = u_res.y;
  float x = u_pad + a_pos.x * (W - 2.0 * u_pad);
  float y = (H - u_pad) - a_pos.y * (H - 2.0 * u_pad);
  gl_Position = vec4(x / W * 2.0 - 1.0, -(y / H * 2.0 - 1.0), 0.0, 1.0);
  float radius = abs(a_flags - 1.0) < 0.5 ? 3.5 : a_size;
  gl_PointSize = (radius * 2.0 + 4.0) * u_dpr;
  int id = gl_VertexID + 1;
  v_id = vec3(float(id & 255), float((id >> 8) & 255), float((id >> 16) & 255)) / 255.0;
}`

const PICK_FS = `#version 300 es
precision highp float;
flat in vec3 v_id;
out vec4 frag;
void main() {
  vec2 d = gl_PointCoord - 0.5;
  if (dot(d, d) > 0.25) discard;
  frag = vec4(v_id, 1.0);
}`

function compile(gl, type, src) {
  const sh = gl.createShader(type)
  gl.shaderSource(sh, src)
  gl.compileShader(sh)
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(sh) || 'shader')
  return sh
}
function program(gl, vs, fs) {
  const p = gl.createProgram()
  gl.attachShader(p, compile(gl, gl.VERTEX_SHADER, vs))
  gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, fs))
  gl.linkProgram(p)
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) || 'link')
  return p
}

// A 2x2 atlas: 0 oppose chevron, 1 support chevron, 2 neutral bar, 3 hollow ring.
// Matte ink (white here, tinted in-shader), drawn at 64px for clean minification.
function buildAtlas() {
  const C = 64
  const cv = document.createElement('canvas')
  cv.width = C * 2; cv.height = C * 2
  const g = cv.getContext('2d')
  g.strokeStyle = '#fff'; g.fillStyle = '#fff'; g.lineCap = 'round'; g.lineJoin = 'round'
  const chevron = (ox, oy, up) => {
    g.lineWidth = 10
    const m = 16, cx = ox + C / 2
    g.beginPath()
    if (up) { g.moveTo(ox + m, oy + C - m); g.lineTo(cx, oy + m); g.lineTo(ox + C - m, oy + C - m) }
    else { g.moveTo(ox + m, oy + m); g.lineTo(cx, oy + C - m); g.lineTo(ox + C - m, oy + m) }
    g.stroke()
  }
  chevron(0, 0, false)      // cell 0: oppose, points down
  chevron(C, 0, true)       // cell 1: support, points up
  g.lineWidth = 10          // cell 2: neutral bar
  g.beginPath(); g.moveTo(16, C + C / 2); g.lineTo(C - 16, C + C / 2); g.stroke()
  g.lineWidth = 7           // cell 3: hollow ring
  g.beginPath(); g.arc(C + C / 2, C + C / 2, C / 2 - 12, 0, Math.PI * 2); g.stroke()
  return cv
}

function createWebGL(canvas) {
  const gl = canvas.getContext('webgl2', { antialias: true, premultipliedAlpha: false, alpha: true })
  if (!gl) return null

  const draw = program(gl, DRAW_VS, DRAW_FS)
  const pick = program(gl, PICK_VS, PICK_FS)
  const u = (p, n) => gl.getUniformLocation(p, n)
  const dU = { res: u(draw, 'u_res'), pad: u(draw, 'u_pad'), time: u(draw, 'u_time'), dur: u(draw, 'u_dur'), dpr: u(draw, 'u_dpr'), sel: u(draw, 'u_selected'), atlas: u(draw, 'u_atlas') }
  const pU = { res: u(pick, 'u_res'), pad: u(pick, 'u_pad'), dpr: u(pick, 'u_dpr') }

  const vao = gl.createVertexArray()
  gl.bindVertexArray(vao)
  const buf = { pos: gl.createBuffer(), from: gl.createBuffer(), to: gl.createBuffer(), delay: gl.createBuffer(), size: gl.createBuffer(), flags: gl.createBuffer() }
  const attr = (loc, b, n) => {
    gl.bindBuffer(gl.ARRAY_BUFFER, b)
    gl.enableVertexAttribArray(loc)
    gl.vertexAttribPointer(loc, n, gl.FLOAT, false, 0, 0)
  }
  attr(0, buf.pos, 2); attr(1, buf.from, 1); attr(2, buf.to, 1); attr(3, buf.delay, 1); attr(4, buf.size, 1); attr(5, buf.flags, 1)
  gl.bindVertexArray(null)

  const tex = gl.createTexture()
  gl.bindTexture(gl.TEXTURE_2D, tex)
  // No UNPACK_FLIP_Y: gl_PointCoord is top-down and the atlas canvas is top-down,
  // so leaving the texture unflipped keeps cell rows and glyph orientation upright.
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, buildAtlas())
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)

  // Offscreen ID-colour buffer for picking.
  const pickFbo = gl.createFramebuffer()
  const pickTex = gl.createTexture()
  const pickBuf = new Uint8Array(4)
  let count = 0, dpr = 1, pad = 28

  const resizePickTarget = () => {
    gl.bindTexture(gl.TEXTURE_2D, pickTex)
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, canvas.width, canvas.height, 0, gl.RGBA, gl.UNSIGNED_BYTE, null)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST)
    gl.bindFramebuffer(gl.FRAMEBUFFER, pickFbo)
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, pickTex, 0)
    gl.bindFramebuffer(gl.FRAMEBUFFER, null)
  }

  gl.enable(gl.BLEND)
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)

  return {
    webgl: true,
    setViewport(w, h, ratio, p) {
      dpr = ratio; pad = p * ratio
      canvas.width = Math.max(1, Math.round(w * ratio))
      canvas.height = Math.max(1, Math.round(h * ratio))
      canvas.style.width = w + 'px'; canvas.style.height = h + 'px'
      resizePickTarget()
    },
    setData(marks) {
      count = marks.length
      const pos = new Float32Array(count * 2)
      const size = new Float32Array(count)
      const flags = new Float32Array(count)
      for (let i = 0; i < count; i++) {
        const m = marks[i]
        pos[i * 2] = m._x; pos[i * 2 + 1] = m._y
        size[i] = markRadius(m)
        flags[i] = isRefused(m) ? 1 : isSynthetic(m) ? 2 : 0
      }
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.pos); gl.bufferData(gl.ARRAY_BUFFER, pos, gl.STATIC_DRAW)
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.size); gl.bufferData(gl.ARRAY_BUFFER, size, gl.STATIC_DRAW)
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.flags); gl.bufferData(gl.ARRAY_BUFFER, flags, gl.STATIC_DRAW)
    },
    setRound(from, to, delay) {
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.from); gl.bufferData(gl.ARRAY_BUFFER, from, gl.DYNAMIC_DRAW)
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.to); gl.bufferData(gl.ARRAY_BUFFER, to, gl.DYNAMIC_DRAW)
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.delay); gl.bufferData(gl.ARRAY_BUFFER, delay, gl.DYNAMIC_DRAW)
    },
    render(timeSec, durSec, selected) {
      gl.viewport(0, 0, canvas.width, canvas.height)
      gl.clearColor(0, 0, 0, 0)
      gl.clear(gl.COLOR_BUFFER_BIT)
      if (!count) return
      gl.useProgram(draw)
      gl.bindVertexArray(vao)
      gl.uniform2f(dU.res, canvas.width, canvas.height)
      gl.uniform1f(dU.pad, pad)
      gl.uniform1f(dU.time, timeSec)
      gl.uniform1f(dU.dur, durSec)
      gl.uniform1f(dU.dpr, dpr)
      gl.uniform1i(dU.sel, selected == null ? -1 : selected)
      gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(dU.atlas, 0)
      gl.drawArrays(gl.POINTS, 0, count)
      gl.bindVertexArray(null)
    },
    pick(cssX, cssY) {
      if (!count) return -1
      gl.bindFramebuffer(gl.FRAMEBUFFER, pickFbo)
      gl.viewport(0, 0, canvas.width, canvas.height)
      gl.disable(gl.BLEND)
      gl.clearColor(0, 0, 0, 0)
      gl.clear(gl.COLOR_BUFFER_BIT)
      gl.useProgram(pick)
      gl.bindVertexArray(vao)
      gl.uniform2f(pU.res, canvas.width, canvas.height)
      gl.uniform1f(pU.pad, pad)
      gl.uniform1f(pU.dpr, dpr)
      gl.drawArrays(gl.POINTS, 0, count)
      const gx = Math.round(cssX * dpr)
      const gy = Math.round(canvas.height - cssY * dpr)
      gl.readPixels(gx, gy, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pickBuf)
      gl.bindVertexArray(null)
      gl.bindFramebuffer(gl.FRAMEBUFFER, null)
      gl.enable(gl.BLEND)
      const id = pickBuf[0] | (pickBuf[1] << 8) | (pickBuf[2] << 16)
      return id ? id - 1 : -1
    },
    dispose() {
      gl.getExtension('WEBGL_lose_context')?.loseContext()
    },
  }
}

// 2D fallback: same public API, CPU-interpolated sweep. Keeps the map alive when
// WebGL2 is missing. Draws the same chevron/bar/ring glyphs.
function create2D(canvas) {
  const ctx = canvas.getContext('2d')
  let marks = [], W = 1, H = 1, pad = 28
  let from = new Float32Array(0), to = new Float32Array(0), delay = new Float32Array(0)
  const px = (v) => pad + v * (W - 2 * pad)
  const py = (v) => (H - pad) - v * (H - 2 * pad)
  const glyph = (x, y, r, s, refused) => {
    ctx.beginPath()
    if (refused) { ctx.arc(x, y, r, 0, Math.PI * 2); return }
    if (s < -0.15) { ctx.moveTo(x - r, y - r * 0.6); ctx.lineTo(x, y + r * 0.7); ctx.lineTo(x + r, y - r * 0.6) }
    else if (s > 0.15) { ctx.moveTo(x - r, y + r * 0.6); ctx.lineTo(x, y - r * 0.7); ctx.lineTo(x + r, y + r * 0.6) }
    else { ctx.moveTo(x - r, y); ctx.lineTo(x + r, y) }
  }
  return {
    webgl: false,
    setViewport(w, h, ratio, p) {
      W = w; H = h; pad = p
      canvas.width = Math.round(w * ratio); canvas.height = Math.round(h * ratio)
      canvas.style.width = w + 'px'; canvas.style.height = h + 'px'
      ctx.setTransform(ratio, 0, 0, ratio, 0, 0)
    },
    setData(m) { marks = m },
    setRound(f, t, d) { from = f; to = t; delay = d },
    render(timeSec, durSec, selected) {
      ctx.clearRect(0, 0, W, H)
      for (let i = 0; i < marks.length; i++) {
        const m = marks[i]
        const t = Math.max(0, Math.min(1, (timeSec - delay[i]) / durSec))
        const stance = from[i] + (to[i] - from[i]) * easeOutQuint(t)
        const x = px(m._x), y = py(m._y), sel = i === selected
        if (isRefused(m)) {
          ctx.strokeStyle = 'rgb(128,128,133)'; ctx.globalAlpha = sel ? 0.7 : 0.45; ctx.lineWidth = 1
          glyph(x, y, sel ? 5 : 3.5, 0, true); ctx.stroke(); continue
        }
        const [r, g, b] = stanceRGB(stance)
        ctx.strokeStyle = `rgb(${r | 0},${g | 0},${b | 0})`
        ctx.globalAlpha = isSynthetic(m) ? 0.5 : 1
        ctx.lineWidth = sel ? 3 : 2
        glyph(x, y, markRadius(m) + (sel ? 2 : 0), stance, false); ctx.stroke()
      }
      ctx.globalAlpha = 1
    },
    pick(cssX, cssY) {
      let best = -1, bestD = 16 * 16
      for (let i = 0; i < marks.length; i++) {
        const dx = px(marks[i]._x) - cssX, dy = py(marks[i]._y) - cssY, d = dx * dx + dy * dy
        if (d < bestD) { bestD = d; best = i }
      }
      return best
    },
    dispose() {},
  }
}

export function createMarkRenderer(canvas) {
  try {
    const r = createWebGL(canvas)
    if (r) return r
  } catch { /* fall through to 2D */ }
  return create2D(canvas)
}
