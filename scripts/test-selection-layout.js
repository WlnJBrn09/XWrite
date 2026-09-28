#!/usr/bin/env node
/**
 * Tests for the shipped selection layout helpers and popover overflow rules.
 * Exercises static/selection-layout.js (the browser entry) — no stubs.
 */
'use strict';

const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const layoutPath = path.join(root, 'static', 'selection-layout.js');
const cssPath = path.join(root, 'static', 'style.css');
const htmlPath = path.join(root, 'static', 'index.html');
const scriptPath = path.join(root, 'static', 'script.js');

let failed = 0;
let passed = 0;

function assert(cond, msg) {
  if (cond) {
    passed++;
    console.log('  PASS  ' + msg);
  } else {
    failed++;
    console.error('  FAIL  ' + msg);
  }
}

function assertClose(actual, expected, eps, msg) {
  const ok = Math.abs(actual - expected) <= eps;
  assert(ok, `${msg} (got ${actual}, expected ~${expected})`);
}

console.log('=== Selection layout tests ===\n');

const Layout = require(layoutPath);
const css = fs.readFileSync(cssPath, 'utf8');
const html = fs.readFileSync(htmlPath, 'utf8');
const script = fs.readFileSync(scriptPath, 'utf8');

console.log('1. Wiring');
assert(html.includes('selection-layout.js'), 'index.html loads selection-layout.js');
assert(script.includes('XSuiteSelectionLayout'), 'script uses XSuiteSelectionLayout');
assert(html.includes('id="text-loupe"') && html.includes('id="text-loupe-content"'), 'text loupe markup present');
assert(!/id="paper"[^>]*class="[^"]*panel/.test(html), 'paper is a document surface, not chrome');

// ── 9b. Loupe + floating toolbar layout math ─────────────
console.log('\n9b. Loupe / floating toolbar layout helpers');
assert(typeof Layout.loupeContentOffset === 'function', 'exports loupeContentOffset');
assert(typeof Layout.floatingToolbarLayout === 'function', 'exports floatingToolbarLayout');
const lo = Layout.loupeContentOffset(40, 30, 120, 2);
assertClose(lo.tx, 60 - 80, 0.05, 'loupe tx centers focusX under scale');
assertClose(lo.ty, 60 - 60, 0.05, 'loupe ty centers focusY under scale');
assertClose(lo.scale, 2, 0.001, 'loupe scale preserved');
const ft = Layout.floatingToolbarLayout(
  { left: 200, top: 300, width: 100, height: 20, bottom: 320 },
  { left: 0, top: 0, width: 800, height: 1000 },
  { barHeight: 44, gap: 8, pad: 8, halfWidth: 90 }
);
assert(ft.top < 300, 'toolbar sits above selection when room allows');
assert(ft.left > 90 && ft.left < 800 - 90, 'toolbar left clamped in host');
const ftLow = Layout.floatingToolbarLayout(
  { left: 100, top: 10, width: 80, height: 16, bottom: 26 },
  { left: 0, top: 0, width: 400, height: 200 },
  { barHeight: 44, gap: 8, pad: 8, halfWidth: 60 }
);
assert(ftLow.top >= 8, 'toolbar flips below when no room above');

// ── 10. Overflow must not clip chrome popovers ───────────
console.log('\n10. Overflow rules for menus / pickers (skeptic-fixed)');
const panelMatch = css.match(/\.panel\s*\{([^}]+)\}/);
assert(!!panelMatch, 'found base .panel rule');
assert(!/overflow\s*:\s*hidden/.test(panelMatch ? panelMatch[1] : ''), 'base .panel does not clip popovers');

// toolbar-pill must not force non-visible overflow that collapses y→auto
const pillMatch = css.match(/\.toolbar-pill\s*\{([^}]+)\}/);
assert(!!pillMatch, 'found .toolbar-pill rule');
const pillDecls = pillMatch[1]
  .replace(/\/\*[\s\S]*?\*\//g, '')
  .split(';')
  .map((s) => s.trim())
  .filter(Boolean);
assert(
  pillDecls.some((d) => /^overflow\s*:\s*visible\s*$/.test(d)),
  'toolbar-pill overflow:visible so menus can escape'
);
assert(
  !pillDecls.some((d) => /^overflow-x\s*:\s*auto\s*$/.test(d)),
  'toolbar-pill itself is not overflow-x:auto (scroll is on .toolbar-scroll)'
);
assert(/\.toolbar-scroll\s*\{/.test(css), 'inner .toolbar-scroll scroller exists');
const scrollMatch = css.match(/\.toolbar-scroll\s*\{([^}]+)\}/);
assert(!!scrollMatch, 'found .toolbar-scroll rule');
assert(/overflow-x\s*:\s*auto/.test(scrollMatch[1]), 'toolbar-scroll has overflow-x:auto');

// glass-menu: overflow-y auto without overflow:hidden shorthand
const menuMatch = css.match(/\.glass-menu\s*\{([^}]+)\}/);
assert(!!menuMatch, 'found .glass-menu rule');
const menuDecls = menuMatch[1]
  .replace(/\/\*[\s\S]*?\*\//g, '')
  .split(';')
  .map((s) => s.trim())
  .filter(Boolean);
assert(
  menuDecls.some((d) => /^overflow-y\s*:\s*auto\s*$/.test(d)),
  'glass-menu overflow-y:auto'
);
assert(
  !menuDecls.some((d) => /^overflow\s*:\s*hidden\s*$/.test(d)),
  'glass-menu rule has no overflow:hidden shorthand'
);

// font-menu must live outside toolbar scroller in markup
assert(
  /toolbar-scroll[\s\S]*?<\/div>\s*<\/div>\s*<div id="font-menu"/.test(html) ||
    (html.includes('toolbar-scroll') &&
      html.indexOf('id="font-menu"') > html.indexOf('toolbar-scroll') &&
      html.indexOf('id="font-menu"') > html.indexOf('</div>', html.indexOf('id="toolbar"'))),
  'font-menu is outside toolbar-scroll / not nested in scroller'
);
assert(
  script.includes('positionMenu') || script.includes('positionFontMenu'),
  'script positions portaled font menu'
);

// ── Summary ──────────────────────────────────────────────
console.log('\n=== Results ===');
console.log(`passed=${passed} failed=${failed}`);
process.exitCode = failed > 0 ? 1 : 0;
