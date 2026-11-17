/* Local pencil drawings. Frametell owns the paths, action clock and seeking. */
(function () {
  function sketch(P, name) {
    if (!['book', 'scroll', 'person', 'tree', 'mountain'].includes(name)) return null;
    const Q = P.layer();
    const path = (d, color = 'ink', sw = 1.7) => Q.path(d, { color, sw, single: true, rough: .7, bow: .35, ease: PV.ease.lin });
    const line = (x, y, xx, yy, color = 'muted', sw = .8) => path(`M${x} ${y}L${xx} ${yy}`, color, sw);
    if (name === 'book') {
      path('M0 -94 Q-74 -124 -148 -98 L-144 102 Q-70 83 0 124');
      path('M0 -94 Q74 -124 148 -98 L144 102 Q70 83 0 124');
      path('M0 -94 Q-3 10 0 124', 'soft', 2);
      path('M-148 -98 L-154 113 Q-76 96 0 137 Q76 96 154 113 L148 -98', 'soft');
      path('M-154 113 L-155 123 Q-78 109 0 145 Q78 109 155 123 L154 113', 'soft');
      for (const side of [-1, 1]) {
        for (let i = 0; i < 13; i++) {
          const y = -76 + i * 13;
          path(`M${side * 13} ${y + 12} Q${side * 68} ${y - 7} ${side * 128} ${y + 3}`, 'soft', .75);
        }
        for (let i = 0; i < 15; i++) {
          const x = side * (12 + i * 9);
          line(x, 120 - Math.abs(x) * .15, x + side * 6, 126 - Math.abs(x) * .15);
        }
      }
    } else if (name === 'scroll') {
      path('M-112 -127 Q0 -136 112 -126 L112 130 Q0 120 -112 130Z');
      path('M-112 -127 Q-136 -148 -140 -125 Q-139 -107 -112 -112', 'soft');
      path('M112 -126 Q139 -146 142 -124 Q140 -107 112 -112', 'soft');
      path('M-112 130 Q-136 149 -140 129 Q-137 111 -112 114', 'soft');
      path('M112 130 Q140 150 143 130 Q140 112 112 114', 'soft');
      for (let i = 0; i < 15; i++) path(`M-87 ${-95 + i * 13} Q0 ${-100 + i * 13} ${78 - i % 3 * 12} ${-95 + i * 13}`, 'soft', .7);
      for (let i = 0; i < 24; i++) {
        const x = -105 + i * 9;
        line(x, 124, x + 7, 131);
      }
    } else if (name === 'person') {
      path('M-22 -106 Q-36 -133 -13 -146 Q15 -160 29 -137 Q40 -114 18 -99');
      path('M-22 -106 Q-2 -94 18 -99', 'soft');
      path('M-17 -101 Q-49 -82 -48 -17 L-88 125 Q-24 145 76 124 L43 -23 Q45 -76 18 -101');
      path('M-34 -76 Q-43 -43 -12 -15 L32 -39');
      path('M-31 -80 Q-30 -49 -9 -34 L26 -47', 'soft');
      path('M-88 125 L-82 143 L-38 143 M25 134 L35 145 L78 140', 'soft');
      path('M-17 -101 Q-9 -78 1 -61 Q9 -82 18 -101', 'soft');
      path('M28 -48 Q41 -53 45 -44 L33 -37 M-9 -17 L-3 -22', 'soft', 1);
      for (let i = 0; i < 10; i++) path(`M${-28 + i * 7} ${-26 + i % 3 * 9} Q${-55 + i * 15} 64 ${-61 + i * 15} 119`, 'soft', .85);
      for (let i = 0; i < 25; i++) {
        const y = -10 + i * 5;
        line(-43 - i * .9, y, -30 - i * .8, y + 12);
        if (i > 10) line(43 + i * .6, y, 53 + i * .6, y + 11, 'soft', .65);
      }
    } else if (name === 'mountain') {
      path('M-172 132 L-133 62 L-111 38 L-94 -31 L-62 -86 L-25 -34 L9 -5');
      path('M9 -5 L29 -71 L65 -151 L92 -108 L103 -63 L129 12 L169 132');
      path('M-62 -86 L-53 -15 L-21 28 L-10 99 M65 -151 L48 -62 L76 -6 L83 88', 'soft', 1.2);
      for (let i = 0; i < 12; i++) {
        const x = -122 + i * 23, y = 58 + Math.sin(i * 1.7) * 35;
        path(`M${x} ${y} q-10 15 -7 30 t-8 24`, 'soft', .85);
      }
      for (let i = 0; i < 36; i++) {
        const y = -130 + i * 7, x = 64 + i * .4;
        line(x, y, x + 8 + i * .9, y + 17, i % 4 ? 'muted' : 'soft', .7);
        if (i > 9) line(-62 - (i - 9) * 2.9, y + 21, -49 - (i - 9) * 2.7, y + 34);
      }
    } else {
      path('M-28 148 Q-17 87 -12 43 Q2 -18 -10 -61 L3 -137');
      path('M-10 148 Q-4 94 3 42 Q16 -15 7 -62 L10 -135', 'soft');
      const branches = [[[-12, 42], [-65, 1], [-119, -13]], [[3, -10], [53, -51], [116, -64]], [[-9, -61], [-54, -103], [-99, -111]], [[4, -95], [39, -125], [78, -139]]];
      for (const points of branches) path(`M${points.map(([x, y]) => `${x} ${y}`).join('L')}`, 'soft', 1.4);
      for (let i = 0; i < 9; i++) path(`M${-24 + i * 2.8} ${53 + i % 3 * 14} q-5 29 -4 62`, 'soft', .75);
      for (const points of branches) {
        const [x, y] = points.at(-1);
        for (let j = 0; j < 20; j++) {
          const a = j * 2.4, px = x + Math.cos(a) * (12 + j % 4 * 7), py = y + Math.sin(a) * (9 + j % 5 * 3);
          line(px, py, px - 12, py - 13, j % 3 ? 'soft' : 'muted', .7);
          line(px, py, px + 8, py - 15, 'muted', .65);
        }
      }
    }
    return Q;
  }

  // Use the existing registered draw actions; no second animation clock or pixel cache.
  function sequence(P, first, at, duration) {
    const strokes = P.sh.acts.slice(first).filter(action => action.kind === 'draw');
    const lengths = strokes.map(action => [...action.g.querySelectorAll('path')].reduce((sum, path) => sum + path.getTotalLength() + 1, 0));
    const total = lengths.reduce((sum, length) => sum + length, 0);
    let elapsed = 0;
    strokes.forEach((action, i) => {
      const span = duration * lengths[i] / total;
      action.at = at + elapsed;
      action.dur = span * .96;
      elapsed += span;
    });
  }

  function mount(P) {
    const strokes = P.sh.acts.filter(action => action.kind === 'draw' && action.pen);
    const tip = PV.s('g', { class: 'pencil-tip', 'pointer-events': 'none' }, P.hud.svg);
    PV.s('path', { d: 'M4 -15L31 -76Q34 -81 39 -77L44 -72Q47 -68 44 -63L15 -8Z', fill: '#73767a', stroke: '#42454a', 'stroke-width': '1' }, tip);
    PV.s('path', { d: 'M9 -14L37 -76 M14 -10L43 -69', fill: 'none', stroke: '#b9bdc2', 'stroke-width': '1.5' }, tip);
    PV.s('path', { d: 'M0 0L4 -15L15 -8Z', fill: '#d6c5a5', stroke: '#847761', 'stroke-width': '.6' }, tip);
    PV.s('path', { d: 'M0 0L2 -6L6 -3Z', fill: '#303238' }, tip);
    P.tick(t => {
      const action = strokes.findLast(stroke => t > stroke.at && t < stroke.at + stroke.dur);
      const point = action?.tip();
      tip.style.visibility = point ? 'visible' : 'hidden';
      if (point) {
        const local = new DOMPoint(point.x, point.y).matrixTransform(P.hud.svg.getScreenCTM().inverse());
        tip.setAttribute('transform', `translate(${local.x},${local.y})`);
      }
    });
  }

  if (SCRIPT.META.look) {
    window.STYLE.id = 'shuyin-pencil';
    window.STYLE.background = () => {
      const fibers = Array.from({ length: 120 }, (_, i) => `<path d="M${i * 83 % 480} ${i * 137 % 480}l${2 + i % 5} ${i % 3 - 1}"/>`).join('');
      return `url("data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="480" height="480"><g stroke="#76787b" stroke-opacity=".09" stroke-width=".55">${fibers}</g></svg>`)}"),#fbfaf7`;
    };
    const css = document.createElement('style');
    css.textContent = '#vignette{background:none}.page{will-change:auto}.ink path{mix-blend-mode:multiply}.pencil-tip path{mix-blend-mode:normal}.subs span{letter-spacing:.035em}';
    document.head.appendChild(css);
  }
  window.SHUYIN_PENCIL = { sketch, sequence, mount };
})();
