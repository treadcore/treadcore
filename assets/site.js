/* treadcore.org: enhancements for browsers that run JavaScript. Every
   page is complete without this file.

   - marks <html> with .js, so panels can rise into place as they come
     into view (site.css, "motion")
   - a theme switch in the metal bar: auto follows the system, day and
     night override it, remembered per browser
   - the bar's shadow once the page scrolls, the hero plate's drift (--s)
   - in browsers without scroll-driven CSS animations, each showcase
     screen's trip (--in, --out), eased; html.sda where they exist */
(function () {
  'use strict';
  var root = document.documentElement;
  root.classList.add('js');
  function load(k, d) { try { var v = localStorage.getItem('tc-' + k); return v === null ? d : v; } catch (e) { return d; } }
  function save(k, v) { try { localStorage.setItem('tc-' + k, v); } catch (e) { /* storage may be unavailable */ } }

  // ----- theme -----
  var THEMES = ['auto', 'light', 'dark'], LABEL = { auto: 'Auto', light: 'Day', dark: 'Night' };
  function apply(t) { if (t === 'auto') root.removeAttribute('data-theme'); else root.setAttribute('data-theme', t); }
  var theme = load('theme', 'auto');
  if (THEMES.indexOf(theme) < 0) theme = 'auto';
  apply(theme);
  var list = document.querySelector('.nav ul');
  if (list) {
    var li = document.createElement('li'), btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'btn';
    function label() { btn.textContent = LABEL[theme]; btn.setAttribute('aria-label', 'Colour theme: ' + LABEL[theme] + '. Change'); }
    label();
    btn.addEventListener('click', function () {
      theme = THEMES[(THEMES.indexOf(theme) + 1) % THEMES.length];
      apply(theme); save('theme', theme); label();
      window.dispatchEvent(new Event('themechange'));
    });
    li.appendChild(btn);
    list.appendChild(li);
  }

  // ----- scrolling -----
  var sda = !!(window.CSS && CSS.supports && CSS.supports('animation-timeline: view()'));
  if (sda) root.classList.add('sda');
  var motion = !(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  var nav = document.querySelector('.nav');
  var plate = document.querySelector('.hero .plate');
  var raf = window.requestAnimationFrame;
  var shots = [], i;
  if (motion && !sda) {
    var els = document.querySelectorAll('.show');
    for (i = 0; i < els.length; i++) shots.push({ el: els[i], now: [1, 0], aim: [1, 0] });
  }
  function phase(a, b, x) { return b <= a ? (x >= b ? 1 : 0) : Math.max(0, Math.min(1, (x - a) / (b - a))); }
  var easing = false, lastT = 0;
  function settle(t) {
    var dt = Math.min(64, t - (lastT || t)), k = 1 - Math.exp(-dt / 90), busy = false;
    lastT = t;
    for (i = 0; i < shots.length; i++) {
      for (var j = 0; j < 2; j++) {
        var c = shots[i], d = c.aim[j] - c.now[j];
        if (Math.abs(d) < 0.002) { if (!d) continue; c.now[j] = c.aim[j]; } else { c.now[j] += d * k; busy = true; }
        c.el.style.setProperty(j ? '--out' : '--in', c.now[j].toFixed(3));
      }
    }
    if (busy) raf(settle); else { easing = false; lastT = 0; }
  }
  var queued = false;
  function update() {
    queued = false;
    var y = window.pageYOffset || root.scrollTop || 0, vh = window.innerHeight || 1;
    if (nav) nav.classList.toggle('scrolled', y > 8);
    if (!motion) return;
    if (plate) plate.style.setProperty('--s', Math.max(0, Math.min(1, y / (vh * 0.9))).toFixed(3));
    for (i = 0; i < shots.length; i++) {
      var r = shots[i].el.getBoundingClientRect();
      if (r.bottom < -vh || r.top > vh * 2) continue;
      var s = vh - r.top;
      shots[i].aim = [phase(0, vh * 0.7, s), phase(r.height + vh * 0.4, r.height + vh, s)];
    }
    if (shots.length && raf && !easing) { easing = true; raf(settle); }
  }
  function onScroll() { if (queued) return; queued = true; if (raf) raf(update); else update(); }
  window.addEventListener('scroll', onScroll, { passive: true });
  window.addEventListener('resize', onScroll);
  update();

  // ----- panels rise into place -----
  var items = document.querySelectorAll('.reveal');
  if ('IntersectionObserver' in window) {
    var io = new IntersectionObserver(function (es) {
      es.forEach(function (e) { if (e.isIntersecting) { e.target.classList.add('in'); io.unobserve(e.target); } });
    }, { rootMargin: '0px 0px -6% 0px', threshold: 0.06 });
    Array.prototype.forEach.call(items, function (el) { io.observe(el); });
  } else {
    Array.prototype.forEach.call(items, function (el) { el.classList.add('in'); });
  }
})();
