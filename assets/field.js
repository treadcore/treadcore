/* treadcore.org: the battlefield behind every page. Decoration only.

   An island in the game's colours (grass, forest, water, roads and a
   bridge, pillboxes and depots), drawn a pixel a tile and scaled up
   without smoothing, as the window draws it. Tanks of two sides drive
   between points on land; each keeps a recon drone circling over it;
   now and then one lobs a shell at the nearest foe, which arcs, lands,
   flashes and leaves a crater that fades. The page's scroll pans the
   field slowly. The colours are the stylesheet's (--f-*), read again
   when the theme changes. With motion unwelcome, one still frame; when
   the tab is hidden, nothing. */
(function () {
  'use strict';
  var cv = document.querySelector('canvas.field');
  if (!cv || !cv.getContext) return;
  var ctx = cv.getContext('2d');
  var still = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var PX = 3;          // css pixels a canvas pixel
  var TS = 3;          // canvas pixels a tile
  var COLS = 150, ROWS = 120;
  var C = {};
  function colours() {
    var st = getComputedStyle(document.documentElement);
    ['water', 'deep', 'grass', 'forest', 'road', 'sand', 'tank', 'tank-dark', 'foe', 'drone', 'shell'].forEach(function (k) {
      C[k] = st.getPropertyValue('--f-' + k).trim() || '#000';
    });
  }

  // ----- the island: seeded value noise, a few octaves, sloped to the sea at the edges -----
  var seed = 0x7eadc0de;
  function rnd() { seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5; return ((seed >>> 0) % 100000) / 100000; }
  function lattice(n) { var g = []; for (var i = 0; i < n * n; i++) g.push(rnd()); return g; }
  var G1 = lattice(32), G2 = lattice(32);
  function noise(g, x, y) {
    var xi = Math.floor(x), yi = Math.floor(y), xf = x - xi, yf = y - yi;
    function at(a, b) { return g[((b & 31) * 32) + (a & 31)]; }
    var u = xf * xf * (3 - 2 * xf), v = yf * yf * (3 - 2 * yf);
    return (at(xi, yi) * (1 - u) + at(xi + 1, yi) * u) * (1 - v) + (at(xi, yi + 1) * (1 - u) + at(xi + 1, yi + 1) * u) * v;
  }
  function fbm(g, x, y) { return noise(g, x, y) * 0.55 + noise(g, x * 2, y * 2) * 0.3 + noise(g, x * 4, y * 4) * 0.15; }
  var WATER = 0, GRASS = 1, FOREST = 2, ROAD = 3, BRIDGE = 4;
  var map = [];
  for (var y = 0; y < ROWS; y++) {
    for (var x = 0; x < COLS; x++) {
      var h = fbm(G1, x / 14, y / 14);
      var t = map.length;
      map[t] = h < 0.5 ? WATER : (fbm(G2, x / 7, y / 7) > 0.56 ? FOREST : GRASS);
    }
  }
  function tile(x, y) { x = Math.floor(x); y = Math.floor(y); return x < 0 || y < 0 || x >= COLS || y >= ROWS ? WATER : map[y * COLS + x]; }
  function land(x, y) { var t = tile(x, y); return t !== WATER; }
  // roads: a few straight runs, bridged where they cross the water
  for (var r = 0; r < 7; r++) {
    var horiz = rnd() < 0.5, at = Math.floor(rnd() * (horiz ? ROWS : COLS)), from = Math.floor(rnd() * 60), len = 40 + Math.floor(rnd() * 70);
    for (var k = from; k < Math.min(from + len, horiz ? COLS : ROWS); k++) {
      var i2 = horiz ? at * COLS + k : k * COLS + at;
      map[i2] = map[i2] === WATER ? BRIDGE : ROAD;
    }
  }
  // pillboxes and depots on land
  var boxes = [];
  for (var b = 0; b < 40 && boxes.length < 18; b++) {
    var bx = Math.floor(rnd() * COLS), by = Math.floor(rnd() * ROWS);
    if (tile(bx, by) === GRASS) boxes.push({ x: bx, y: by, depot: boxes.length % 3 === 0, side: Math.floor(rnd() * 3) });
  }

  var ground = document.createElement('canvas');
  ground.width = COLS; ground.height = ROWS;
  function paintGround() {
    var g = ground.getContext('2d');
    var cols = [C.water, C.grass, C.forest, C.road, C['tank-dark']];
    for (var y = 0; y < ROWS; y++) {
      for (var x = 0; x < COLS; x++) {
        var t = map[y * COLS + x];
        g.fillStyle = t === WATER && fbm(G2, x / 9, y / 9) < 0.4 ? C.deep : cols[t];
        g.fillRect(x, y, 1, 1);
      }
    }
  }

  // ----- the tanks, their drones, their shells -----
  function landPoint() {
    for (var n = 0; n < 200; n++) { var x = rnd() * COLS, y = rnd() * ROWS; if (land(x, y)) return { x: x, y: y }; }
    return { x: COLS / 2, y: ROWS / 2 };
  }
  var tanks = [];
  for (var n = 0; n < 14; n++) {
    var p = landPoint();
    tanks.push({ x: p.x, y: p.y, h: rnd() * 6.28, to: landPoint(), side: n % 2, drone: rnd() * 6.28, reload: 60 + rnd() * 200 });
  }
  var shells = [], marks = [];
  function step() {
    tanks.forEach(function (t) {
      var dx = t.to.x - t.x, dy = t.to.y - t.y, d = Math.hypot(dx, dy);
      if (d < 1) { t.to = landPoint(); return; }
      var want = Math.atan2(dy, dx), turn = Math.atan2(Math.sin(want - t.h), Math.cos(want - t.h));
      t.h += Math.max(-0.05, Math.min(0.05, turn));
      var nx = t.x + Math.cos(t.h) * 0.12, ny = t.y + Math.sin(t.h) * 0.12;
      if (land(nx, ny)) { t.x = nx; t.y = ny; } else { t.to = landPoint(); }
      t.drone += 0.03;
      if (--t.reload <= 0) {
        t.reload = 120 + rnd() * 260;
        var foe = null, best = 40;
        tanks.forEach(function (o) { if (o.side !== t.side) { var od = Math.hypot(o.x - t.x, o.y - t.y); if (od < best) { best = od; foe = o; } } });
        if (foe) shells.push({ x0: t.x, y0: t.y, x1: foe.x + (rnd() - 0.5) * 4, y1: foe.y + (rnd() - 0.5) * 4, f: 0, len: 40 + best * 2 });
      }
    });
    shells = shells.filter(function (s) {
      s.f++;
      if (s.f >= s.len) { marks.push({ x: s.x1, y: s.y1, age: 0 }); return false; }
      return true;
    });
    marks = marks.filter(function (m) { return ++m.age < 400; });
  }

  // ----- drawing -----
  var W = 0, H = 0, scrollY = 0;
  function size() {
    W = Math.ceil(window.innerWidth / PX); H = Math.ceil(window.innerHeight / PX);
    cv.width = W; cv.height = H;
  }
  function view() {
    // the camera: the field's middle, panned by the page's scroll
    var mw = COLS * TS, mh = ROWS * TS;
    var ox = Math.max(0, (mw - W) / 2), span = Math.max(0, mh - H);
    var doc = Math.max(1, document.documentElement.scrollHeight - window.innerHeight);
    return { x: -ox, y: -Math.min(span, span * 0.15 + (scrollY / doc) * span * 0.7) };
  }
  function draw() {
    var v = view();
    ctx.imageSmoothingEnabled = false;
    ctx.fillStyle = C.deep; ctx.fillRect(0, 0, W, H);
    ctx.drawImage(ground, v.x, v.y, COLS * TS, ROWS * TS);
    function X(x) { return Math.round(v.x + x * TS); }
    function Y(y) { return Math.round(v.y + y * TS); }
    marks.forEach(function (m) {
      ctx.globalAlpha = Math.max(0, 1 - m.age / 400) * 0.6;
      ctx.fillStyle = '#1a140c'; ctx.fillRect(X(m.x) - 2, Y(m.y) - 2, 4, 4);
      if (m.age < 6) { ctx.globalAlpha = 1 - m.age / 6; ctx.fillStyle = '#fff3b0'; ctx.fillRect(X(m.x) - 4, Y(m.y) - 4, 8, 8); }
    });
    ctx.globalAlpha = 1;
    boxes.forEach(function (b) {
      var c = b.side === 0 ? C.tank : b.side === 1 ? C.foe : '#8c8170';
      ctx.fillStyle = '#111'; ctx.fillRect(X(b.x) - 2, Y(b.y) - 2, 5, 5);
      ctx.fillStyle = c; ctx.fillRect(X(b.x) - 1, Y(b.y) - 1, 3, 3);
      if (!b.depot) { ctx.strokeStyle = c; ctx.globalAlpha = 0.35; ctx.beginPath(); ctx.arc(X(b.x) + 0.5, Y(b.y) + 0.5, 9 * TS / 3, 0, 6.28); ctx.stroke(); ctx.globalAlpha = 1; }
    });
    tanks.forEach(function (t) {
      var cx = X(t.x), cy = Y(t.y), body = t.side ? C.foe : C.tank;
      ctx.save(); ctx.translate(cx, cy); ctx.rotate(t.h);
      ctx.fillStyle = '#111'; ctx.fillRect(-4, -3, 8, 6);
      ctx.fillStyle = body; ctx.fillRect(-3, -2, 6, 4);
      ctx.fillStyle = C['tank-dark']; ctx.fillRect(-1, -1, 3, 2); ctx.fillRect(1, -0.5, 4, 1);
      ctx.restore();
      var dx = cx + Math.cos(t.drone) * 10, dy = cy + Math.sin(t.drone) * 10;
      ctx.strokeStyle = C.drone; ctx.globalAlpha = 0.25; ctx.beginPath(); ctx.arc(cx, cy, 10, 0, 6.28); ctx.stroke();
      ctx.globalAlpha = 1; ctx.fillStyle = C.drone; ctx.fillRect(Math.round(dx) - 1, Math.round(dy) - 1, 2, 2);
    });
    shells.forEach(function (s) {
      var f = s.f / s.len, x = s.x0 + (s.x1 - s.x0) * f, y = s.y0 + (s.y1 - s.y0) * f, lift = Math.sin(f * Math.PI) * 6;
      ctx.fillStyle = C.shell; ctx.fillRect(X(x) - 1, Y(y) - 1 - Math.round(lift), 2, 2);
      ctx.globalAlpha = 0.3; ctx.fillStyle = '#000'; ctx.fillRect(X(x), Y(y), 1, 1); ctx.globalAlpha = 1;
    });
  }

  var last = 0, running = false;
  function frame(t) {
    if (document.hidden) { running = false; return; }
    if (t - last > 33) { last = t; step(); draw(); }
    window.requestAnimationFrame(frame);
  }
  function start() { if (!running && !still && window.requestAnimationFrame) { running = true; window.requestAnimationFrame(frame); } }
  function repaint() { colours(); paintGround(); draw(); }
  colours(); size(); paintGround();
  for (var w = 0; w < 200; w++) step();   // a battle already under way
  draw();
  window.addEventListener('resize', function () { size(); draw(); });
  window.addEventListener('scroll', function () { scrollY = window.pageYOffset || 0; if (still || !running) draw(); }, { passive: true });
  window.addEventListener('themechange', repaint);
  if (window.matchMedia) {
    var mq = window.matchMedia('(prefers-color-scheme: dark)');
    if (mq.addEventListener) mq.addEventListener('change', repaint);
  }
  document.addEventListener('visibilitychange', start);
  start();
})();
