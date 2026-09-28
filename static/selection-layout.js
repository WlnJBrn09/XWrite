/**
 * XWrite — layout helpers for the selection loupe and floating selection toolbar.
 * Pure functions are testable without a browser DOM.
 */
(function (root, factory) {
  const api = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  }
  root.XSuiteSelectionLayout = api;
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
  'use strict';

  function clamp(v, lo, hi) {
    return Math.min(hi, Math.max(lo, v));
  }

  function round2(v) {
    return Math.round(v * 100) / 100;
  }

  /**
   * Position a magnifying loupe content layer so focus (local coords) sits at center.
   * @param {number} focusX - pointer X relative to content origin
   * @param {number} focusY - pointer Y relative to content origin
   * @param {number} loupeSize - loupe diameter in px
   * @param {number} scale - magnification
   * @returns {{ tx: number, ty: number, scale: number }}
   */
  function loupeContentOffset(focusX, focusY, loupeSize, scale) {
    const s = scale == null ? 1.65 : scale;
    const half = (loupeSize == null ? 120 : loupeSize) / 2;
    return {
      tx: round2(half - focusX * s),
      ty: round2(half - focusY * s),
      scale: s,
    };
  }

  /**
   * Place floating selection capsule above a selection rect, clamped in a host rect.
   * All rects in the same coordinate space (e.g. paper-local or viewport).
   */
  function floatingToolbarLayout(selRect, hostRect, opts) {
    const o = opts || {};
    const barH = o.barHeight == null ? 44 : o.barHeight;
    const gap = o.gap == null ? 8 : o.gap;
    const pad = o.pad == null ? 8 : o.pad;
    const halfW = o.halfWidth == null ? 90 : o.halfWidth;
    const midX = selRect.left + selRect.width / 2 - hostRect.left;
    let top = selRect.top - hostRect.top - barH - gap;
    if (top < pad) {
      // Flip below selection when not enough room above
      top = selRect.bottom - hostRect.top + gap;
    }
    top = clamp(top, pad, Math.max(pad, hostRect.height - barH - pad));
    const left = clamp(midX, pad + halfW, Math.max(pad + halfW, hostRect.width - pad - halfW));
    return { left: round2(left), top: round2(top) };
  }

  return {
    loupeContentOffset,
    floatingToolbarLayout,
    clamp,
    round2,
  };
});
