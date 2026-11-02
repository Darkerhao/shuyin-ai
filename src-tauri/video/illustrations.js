/* Reference reading-film drawings. Geometry is preserved; all motion uses the source SRT clock. */
(function () {
  const { K } = PV;

  function sketch(P, name, at, tone = 'ink') {
    const Q = P.layer();
    let n = 0;
    const opt = (extra = {}) => ({ color: tone, sw: 3, at: at + n++ * .065, dur: .65, ...extra });
    const path = (d, o) => Q.path(d, opt(o));
    const line = (a, b, c, d, o) => Q.line(a, b, c, d, opt(o));
    const box = (a, b, c, d, o) => Q.box(a, b, c, d, opt({ r: 4, ...o }));
    const circle = (a, b, d, o) => Q.circle(a, b, d, opt(o));
    const ellipse = (a, b, c, d, o) => Q.ellipse(a, b, c, d, opt(o));
    const writing = (x, y, w, count) => {
      for (let i = 0; i < count; i++) line(x, y + i * 25, x + w - i % 3 * 13, y + i * 25, { color: 'muted', sw: 2 });
    };
    const head = (x, y, d = 50) => circle(x, y, d, { fill: 'paper' });
    switch (name) {
      case 'book':
        path('M0 -99 Q-75 -133 -145 -104 V111 Q-68 88 0 127 Q68 88 145 111 V-104 Q75 -133 0 -99 Z', { fill: 'paper' });
        line(0, -98, 0, 124); writing(-117, -65, 91, 6); writing(25, -65, 91, 6);
        path('M-145 111 L-153 126 Q-64 104 0 143 Q64 104 153 126 L145 111', { color: 'soft' });
        break;
      case 'pen':
        path('M-89 113 L62 -113 L91 -93 L-58 133 Z', { fill: 'yellowF' });
        path('M-89 113 L-104 153 L-58 133 M62 -113 L78 -137 L108 -117 L91 -93');
        path('M-108 158 Q-35 125 14 154 T125 145', { color: 'green', sw: 3.5 });
        break;
      case 'crown':
        path('M-135 -61 L-82 14 L0 -111 L82 14 L135 -61 L104 102 H-104 Z', { color: 'orange', fill: 'yellowF' });
        line(-105, 75, 105, 75, { color: 'red', sw: 4 });
        for (const [x, y] of [[-135, -69], [0, -119], [135, -69]]) circle(x, y, 19, { color: 'orange', fill: 'paper' });
        path('M0 20 L18 40 L0 61 L-18 40 Z', { color: 'red', fill: 'redF' });
        break;
      case 'scroll': case 'document': case 'contract': case 'letter': case 'certificate': case 'ballot':
        box(-112, -144, 224, 284, { fill: name === 'letter' ? 'yellowF' : 'paper', color: name === 'letter' ? 'orange' : tone });
        if (name === 'scroll') {
          ellipse(0, -144, 236, 25, { fill: 'paper' }); ellipse(0, 141, 236, 25, { fill: 'paper' });
        }
        writing(-81, -98, 157, name === 'certificate' ? 4 : 6);
        if (name === 'certificate') {
          circle(44, 67, 61, { color: 'red' });
          path('M-56 48 H-16 V91 H-56 Z M-47 69 L-36 80 L-17 51', { color: 'green' });
        } else if (name === 'ballot') {
          box(-77, 69, 32, 34, { color: 'green' }); Q.check(-60, 84, 30, opt({ color: 'green' }));
        } else if (name === 'contract' || name === 'letter') {
          path('M-73 94 Q-46 56 -22 94 T60 84', { color: 'red' });
          box(66, 86, 28, 28, { color: 'red' });
        }
        break;
      case 'person': case 'servant': case 'guard': case 'kneel':
        head(0, -105, 60);
        path(name === 'kneel' ? 'M-32 -66 L-48 39 L7 68 L75 118 H-63 L-101 80 L-48 39 M-28 -28 L61 6' :
          'M-34 -66 L-62 55 H62 L34 -66 M-36 -36 L-88 52 M36 -36 L88 52 M-27 57 L-33 142 M27 57 L33 142', { fill: 'faint' });
        if (name === 'guard') { line(116, -130, 116, 145, { sw: 5 }); path('M-36 -133 H36 L24 -156 H-24 Z', { fill: 'soft' }); }
        if (name === 'servant') path('M-30 -140 Q0 -166 30 -140 M0 -153 Q42 -157 58 -109', { color: 'soft' });
        break;
      case 'cradle':
        path('M-131 -62 Q0 0 131 -62 L98 83 Q0 133 -98 83 Z', { fill: 'yellowF', color: 'orange' });
        path('M-123 135 Q0 181 123 135 M-81 89 L-99 142 M81 89 L99 142');
        head(0, -41, 56); path('M-55 -3 Q0 45 55 -3', { color: 'muted' });
        break;
      case 'puppet': {
        line(-115, -155, 115, -155, { color: 'soft', sw: 5 });
        for (const x of [-82, 0, 82]) line(x, -151, x * .85, x === 0 ? -82 : 29, { color: 'muted', sw: 1.8 });
        const body = Q.layer();
        body.circle(0, -64, 44, { color: tone, at, dur: .7 });
        body.path('M0 -42 V51 M0 -20 L-70 32 M0 -20 L70 32 M0 51 L-48 129 M0 51 L48 129', { color: tone, sw: 4, at: at + .4, dur: 1.3 });
        Q.move(body, t => ({ rot: -5 + 10 * K(t, at + 1.8, 3) }));
        break;
      }
      case 'palace': case 'gate': case 'shop': case 'office': case 'house':
        if (name === 'palace' || name === 'gate') {
          path('M-165 -61 Q-127 -73 -102 -109 H102 Q127 -73 165 -61 Q111 -46 0 -48 Q-111 -46 -165 -61 Z', { fill: 'yellowF', color: 'orange' });
          path('M-119 -116 Q-88 -129 -63 -159 H63 Q88 -129 119 -116 Z', { fill: 'yellowF', color: 'orange' });
          box(-127, -43, 254, 168, { fill: 'redF', color: 'red' });
          path('M-47 125 V-7 Q0 -57 47 -7 V125', { color: 'soft', fill: 'paper' });
          line(0, -20, 0, 124, { color: 'red' });
          line(-151, 136, 151, 136, { sw: 4 }); line(-167, 149, 167, 149, { color: 'muted' });
          for (const x of [-91, 91]) line(x, -28, x, 122, { color: 'red', sw: 7 });
        } else {
          path(name === 'office' ? 'M-104 138 V-143 H104 V138 Z' : 'M-140 -39 L0 -145 L140 -39 Z', { fill: 'faint' });
          if (name !== 'office') box(-111, -33, 222, 169, { fill: 'paper' });
          for (const x of [-70, 31]) for (const y of [-8, 50]) box(x, y, 38, 36, { fill: 'blueF', color: 'blue' });
          box(-23, 78, 46, 59, { color: 'soft' });
          if (name === 'shop') { path('M-133 -38 H133 L119 1 H-119 Z', { fill: 'yellowF', color: 'orange' }); line(-131, -41, -131, -68); }
        }
        break;
      case 'wall':
        box(-149, -109, 298, 236, { fill: 'faint' });
        for (let row = 0; row < 5; row++) {
          const y = -63 + row * 46; line(-145, y, 145, y, { color: 'muted' });
          for (let col = 0; col < 3; col++) { const x = -100 + col * 100 + row % 2 * 40; if (x < 145) line(x, y - 43, x, y, { color: 'muted' }); }
        }
        break;
      case 'cage':
        path('M-123 -93 Q0 -184 123 -93 V124 H-123 Z', { color: 'soft' });
        for (let x = -84; x <= 84; x += 42) line(x, -108 - (1 - Math.abs(x) / 125) * 27, x, 121, { color: 'soft', sw: 4 });
        line(-123, 48, 123, 48, { color: 'soft' }); ellipse(0, 130, 271, 31, { fill: 'faint' });
        break;
      case 'lock':
        path('M-66 -30 V-79 Q-66 -161 0 -161 Q66 -161 66 -79 V-30', { color: 'soft', sw: 9 });
        box(-110, -31, 220, 177, { fill: 'faint' }); circle(0, 36, 33); path('M-9 51 L-15 93 H15 L9 51');
        break;
      case 'chain':
        for (let i = 0; i < 4; i++) ellipse(-105 + i * 70, -45 + i * 30, 96, 63, { color: 'soft', sw: 5 });
        break;
      case 'trap':
        path('M-138 70 L-97 -50 L-57 70 L-19 -50 L21 70 L59 -50 L99 70 L138 -50', { color: 'red', sw: 4 });
        ellipse(0, 81, 296, 71, { color: 'soft' }); path('M-117 100 L-82 139 L-45 105 L-5 145 L35 107 L76 137 L115 100', { color: 'red' });
        break;
      case 'chair': case 'emptychair':
        box(-75, -142, 150, 180, { color: 'orange', fill: 'yellowF' });
        path('M-107 22 H108 V77 H-107 Z M-86 77 V151 M86 77 V151', { color: 'soft' });
        line(-28, -112, -28, 11, { color: 'orange' }); line(28, -112, 28, 11, { color: 'orange' });
        break;
      case 'robe': case 'suit': case 'uniform': case 'shirt': case 'coat':
        path('M-47 -132 L-143 -72 L-113 4 L-71 -22 L-80 139 H80 L71 -22 L113 4 L143 -72 L47 -132 L0 -91 Z',
          { fill: name === 'robe' ? 'yellowF' : name === 'uniform' ? 'greenF' : 'faint', color: name === 'robe' ? 'orange' : tone });
        line(0, -91, 0, 136, { color: 'soft' });
        if (name === 'robe') { path('M-34 -27 Q13 -87 43 -25 T-18 53 Q-71 100 -21 119', { color: 'red' }); circle(22, -31, 21, { color: 'red' }); }
        if (name === 'suit' || name === 'coat') path('M-47 -132 L-12 -33 L-44 11 M47 -132 L12 -33 L44 11');
        if (name === 'uniform') { box(-59, -39, 40, 38); box(19, -39, 40, 38); line(-66, 72, 66, 72, { color: 'orange', sw: 6 }); }
        break;
      case 'cloth': case 'bundle':
        path('M-134 -94 Q-32 -116 0 -72 Q32 -116 134 -94 L107 113 Q0 139 -107 113 Z', { fill: 'yellowF', color: 'orange' });
        path(name === 'bundle' ? 'M-122 -83 L0 24 L122 -83 M-107 113 L0 24 L107 113 M-31 -23 L33 47 M33 -23 L-31 47' : 'M-80 -89 Q-47 -4 -78 104 M78 -89 Q43 -4 76 104', { color: 'orange' });
        break;
      case 'jar':
        ellipse(0, -102, 158, 35, { color: 'blue' }); path('M-77 -97 Q-57 -44 -100 6 V108 Q0 158 100 108 V6 Q57 -44 77 -97', { color: 'blue', fill: 'blueF' });
        for (const y of [-2, 51, 106]) path(`M-93 ${y} Q0 ${y + 37} 93 ${y}`, { color: 'orange' });
        path('M-42 1 Q0 -46 42 1 Q0 49 -42 1 M-42 74 Q0 27 42 74 Q0 122 -42 74', { color: 'orange' });
        break;
      case 'straw':
        for (let i = 0; i < 36; i++) {
          const x = -123 + i % 9 * 30, y = -103 + Math.floor(i / 9) * 68;
          line(x, y, x + 12, y + 44, { color: 'orange', sw: 3, at: at + i * .035 });
        }
        break;
      case 'coins': case 'jewels':
        for (let i = 0; i < 3; i++) {
          const x = -90 + 90 * i;
          for (let j = 0; j < 3 + i; j++) ellipse(x, 101 - j * 31, 89, 34, { fill: 'yellowF', color: 'orange' });
        }
        if (name === 'jewels') path('M-43 -114 L-12 -142 H45 L76 -114 L18 -57 Z M-43 -114 H76 M-12 -142 L18 -57 L45 -142', { color: 'red', fill: 'redF' });
        break;
      case 'scale': case 'level':
        line(0, -128, 0, 133, { color: 'soft', sw: 5 }); line(-77, 143, 77, 143, { color: 'soft', sw: 5 });
        line(-127, -72, 127, -72); circle(0, -72, 17, { fill: 'orange', color: 'orange' });
        for (const x of [-101, 101]) { path(`M${x} -72 L${x - 48} 36 H${x + 48} Z`, { color: 'muted' }); path(`M${x - 51} 38 Q${x} 96 ${x + 51} 38`, { color: 'green', fill: 'greenF' }); }
        break;
      case 'hierarchy': case 'steps':
        if (name === 'hierarchy') {
          for (let i = 0; i < 3; i++) box(-46 - i * 43, -112 + i * 77, 92 + i * 86, 60, { color: i ? 'soft' : 'orange', fill: i ? 'faint' : 'yellowF' });
        } else path('M-155 136 H-61 V54 H35 V-28 H131 V-112 H163 M-155 157 H177 V-112', { color: 'green', sw: 4 });
        break;
      case 'clock':
        circle(0, 0, 240, { fill: 'paper', color: 'soft' });
        for (let i = 0; i < 12; i++) { const a = i * Math.PI / 6; line(98 * Math.cos(a), 98 * Math.sin(a), 109 * Math.cos(a), 109 * Math.sin(a), { color: 'muted' }); }
        line(0, 0, 0, -79, { sw: 5 }); line(0, 0, 68, 29, { color: 'red', sw: 4 }); circle(0, 0, 13, { fill: 'ink' });
        break;
      case 'calendar':
        box(-111, -122, 222, 261, { fill: 'paper' }); line(-109, -60, 109, -60, { color: 'red' });
        for (const x of [-66, 66]) line(x, -151, x, -101, { color: 'soft', sw: 8 });
        for (let i = 0; i < 12; i++) box(-79 + i % 4 * 45, -25 + Math.floor(i / 4) * 48, 26, 26, { color: 'muted' });
        break;
      case 'hourglass':
        path('M-91 -139 H91 M-91 139 H91 M-69 -133 Q-69 -40 0 0 Q69 40 69 133 M69 -133 Q69 -40 0 0 Q-69 40 -69 133', { color: 'soft' });
        path('M-43 -70 H43 L0 -13 Z M-51 121 L0 68 L51 121 Z', { color: 'orange', fill: 'yellowF' }); line(0, 12, 0, 53, { color: 'orange' });
        break;
      case 'trumpet':
        path('M-113 37 H28 L119 110 V-96 L28 -19 H-113 Z', { fill: 'yellowF', color: 'orange' });
        ellipse(122, 7, 46, 214, { color: 'orange' }); path('M-71 39 V101 Q-71 127 -8 127 H17 V40', { color: 'orange' });
        for (const x of [-54, -18, 18]) line(x, -52, x, -21, { color: 'soft' });
        break;
      case 'bowl': case 'bun':
        if (name === 'bun') { path('M-121 69 Q-134 -39 -54 -71 Q0 -146 54 -71 Q134 -39 121 69 Q0 134 -121 69 Z', { fill: 'yellowF' }); path('M-54 -71 L-14 31 M0 -94 V25 M54 -71 L14 31', { color: 'orange' }); }
        else { path('M-135 -18 Q-116 105 0 112 Q116 105 135 -18', { fill: 'paper', color: 'blue' }); ellipse(0, -19, 272, 61, { color: 'blue' }); ellipse(0, 116, 103, 22, { color: 'blue' }); line(-12, -74, 111, -149, { color: 'orange', sw: 7 }); }
        break;
      case 'bucket':
        path('M-101 -58 L-77 129 Q0 156 77 129 L101 -58', { fill: 'blueF', color: 'blue' }); ellipse(0, -60, 208, 53, { color: 'blue' }); path('M-98 -50 Q0 -237 98 -50', { color: 'soft', sw: 4 });
        break;
      case 'stove':
        box(-120, -12, 240, 150, { fill: 'faint' }); box(-75, 24, 150, 90); path('M-91 -66 H91 L65 -7 H-65 Z', { color: 'blue', fill: 'blueF' }); line(-44, -81, -44, -111, { color: 'muted' }); line(20, -81, 20, -134, { color: 'muted' });
        break;
      case 'pump':
        path('M-138 -49 H19 L57 -91 H123 V-42 H79 L43 -1 H-138 Z', { fill: 'faint' });
        path('M-33 -48 V115 H-105 V-48 M-98 116 H22 M88 -111 H132', { color: 'soft', sw: 5 });
        for (let i = 0; i < 4; i++) path(`M138 ${-17 + i * 34} Q170 ${-27 + i * 34} 190 ${-9 + i * 34}`, { color: 'blue', sw: 2 });
        break;
      case 'eye': case 'lens': case 'glasses':
        if (name === 'eye') { path('M-146 0 Q0 -138 146 0 Q0 138 -146 0 Z'); circle(0, 0, 95, { color: 'blue' }); circle(0, 0, 37, { fill: 'ink' }); }
        else if (name === 'lens') { circle(-32, -34, 191, { color: 'blue' }); line(38, 38, 132, 135, { color: 'soft', sw: 15 }); }
        else { circle(-74, 0, 108, { color: 'orange' }); circle(74, 0, 108, { color: 'orange' }); path('M-21 -8 Q0 -32 21 -8 M-129 -21 L-156 -64 M129 -21 L156 -64', { color: 'orange' }); }
        break;
      case 'mirror': case 'glass':
        ellipse(0, -17, 211, 252, { color: 'soft', fill: 'paper' }); line(0, 109, 0, 151, { color: 'soft' }); ellipse(0, 157, 137, 24, { fill: 'faint' });
        path('M-54 -49 L12 -107 M-55 7 L58 -101', { color: 'blue', sw: 2 });
        break;
      case 'heart':
        path('M0 117 C-207 -16 -96 -160 0 -64 C96 -160 207 -16 0 117 Z', { color: 'red', fill: 'redF' });
        break;
      case 'hands': case 'handshake':
        path('M-150 -23 L-77 -75 L-19 -29 L11 -50 L73 -38 L151 4 L109 85 L66 62 L37 91 L8 71 L-20 88 L-68 44 L-111 61 Z', { fill: 'yellowF', color: 'orange' });
        path('M-19 -29 L-53 8 Q-42 32 -18 21 L12 -4 L69 58 M-29 40 L8 71 M1 22 L37 61', { color: 'orange' });
        break;
      case 'mask':
        path('M-115 -111 Q0 -70 115 -111 V4 Q105 105 0 146 Q-105 105 -115 4 Z', { fill: 'faint' });
        path('M-86 -9 Q-52 -37 -21 -9 M21 -9 Q52 -37 86 -9 M-46 73 Q0 40 46 73', { color: 'soft' });
        break;
      case 'question':
        path('M-65 -83 Q-50 -149 26 -130 Q104 -111 68 -37 Q52 -7 4 18 V64', { color: 'red', sw: 10 }); circle(4, 110, 19, { fill: 'red', color: 'red' });
        break;
      case 'gavel':
        path('M-24 -91 L3 -117 L117 -3 L90 24 Z M-48 -91 L-20 -119 M88 49 L144 -7', { fill: 'faint', color: 'soft' });
        path('M43 -22 L-74 105 L-104 78 L14 -49 Z', { fill: 'yellowF', color: 'orange' }); box(-111, 125, 222, 25, { color: 'soft', fill: 'soft' });
        break;
      case 'key': case 'keys':
        circle(-65, -67, 101, { color: 'orange' }); circle(-65, -67, 41, { color: 'orange' });
        path('M-24 -30 L109 102 L92 120 L68 96 L48 113 L28 91 L47 73 L-43 -16', { color: 'orange', fill: 'yellowF' });
        break;
      case 'map':
        path('M-145 -111 L-48 -134 L48 -105 L145 -126 V116 L48 138 L-48 111 L-145 130 Z', { fill: 'paper' });
        line(-48, -132, -48, 109, { color: 'muted' }); line(48, -104, 48, 135, { color: 'muted' });
        path('M-111 76 Q-113 -33 -36 -24 T110 -60', { color: 'red', sw: 4 });
        break;
      case 'table':
        path('M-146 -54 H146 L122 31 H-122 Z', { fill: 'faint' }); line(-106, 31, -106, 130); line(106, 31, 106, 130); box(-48, -38, 96, 41, { fill: 'paper' });
        break;
      case 'mound': case 'altar':
        path('M-154 135 H154 L111 76 H-111 Z M-111 76 H111 L76 16 H-76 Z M-76 16 H76 V-19 H-76 Z', { fill: 'faint', color: 'soft' });
        if (name === 'altar') { box(-54, -58, 108, 37, { color: 'orange' }); for (const x of [-25, 0, 25]) line(x, -60, x, -114, { color: 'red' }); }
        break;
      case 'boots':
        path('M-91 -112 H-30 L-26 42 Q49 52 65 111 H-110 V18 Z M-110 111 H65 V135 H-110 Z', { color: 'orange', fill: 'yellowF' });
        for (let i = 0; i < 4; i++) line(-65, -71 + i * 27, -28, -67 + i * 27, { color: 'soft' });
        break;
      case 'case':
        box(-129, -74, 258, 203, { fill: 'faint', color: 'soft' }); path('M-43 -75 V-125 H43 V-75', { color: 'orange', sw: 6 });
        for (const x of [-75, 75]) { line(x, -69, x, 125, { color: 'orange' }); box(x - 15, 8, 30, 36, { color: 'orange' }); }
        break;
      case 'train':
        box(-128, -108, 214, 205, { fill: 'faint' }); path('M86 -24 H144 V98 H86', { fill: 'yellowF', color: 'orange' });
        for (const x of [-96, -17]) box(x, -77, 54, 59, { color: 'blue', fill: 'blueF' });
        for (const x of [-76, 64]) { circle(x, 103, 64, { color: 'soft', fill: 'paper' }); circle(x, 103, 18, { color: 'soft' }); }
        line(-147, 146, 158, 146, { color: 'soft' });
        break;
      case 'flag':
        line(-78, -151, -78, 156, { color: 'soft', sw: 5 }); path('M-76 -139 Q16 -171 130 -114 V9 Q24 -49 -76 -15 Z', { fill: 'faint' });
        break;
      case 'ruin':
        path('M-144 123 L-100 51 L-64 87 L-12 0 L41 49 L82 -16 L144 122 Z', { fill: 'faint' });
        path('M-103 29 L-126 -52 L-79 -75 L-48 -7 M29 -17 L48 -111 L91 -94 L79 -38', { color: 'soft' });
        break;
      case 'field': case 'garden': case 'street':
        path('M-151 117 H151 M-119 106 Q-39 8 42 101 M-5 109 Q57 12 139 83', { color: name === 'street' ? 'muted' : 'green' });
        if (name === 'street') { for (let i = 0; i < 4; i++) box(-130 + i * 72, -80 + i % 2 * 30, 49, 150 - i % 2 * 30, { fill: 'faint' }); }
        else { for (const x of [-112, -42, 29, 99]) { line(x, 24, x, 93, { color: 'green' }); path(`M${x} 62 Q${x - 44} 34 ${x - 31} 8 Q${x + 3} 20 ${x} 62 M${x} 43 Q${x + 41} 3 ${x + 32} -20 Q${x} -4 ${x} 43`, { color: 'green', fill: 'greenF' }); } }
        break;
      case 'cloud':
        path('M-100 58 Q-165 50 -144 -14 Q-131 -53 -87 -52 Q-69 -134 12 -102 Q63 -135 99 -74 Q165 -67 153 1 Q182 70 94 70 H-80 Z', { color: 'blue', fill: 'blueF' });
        break;
      case 'river': case 'bridge':
        path('M-159 81 Q-80 58 0 89 T158 81 M-159 119 Q-80 96 0 127 T158 119', { color: 'blue' });
        if (name === 'bridge') { path('M-159 32 Q0 -159 159 32 M-159 60 Q0 -111 159 60', { color: 'soft', sw: 5 }); for (const x of [-115, -57, 0, 57, 115]) line(x, 42 - (1 - Math.abs(x) / 160) * 114, x, 64 - (1 - Math.abs(x) / 160) * 89, { color: 'soft' }); }
        break;
      case 'shears':
        circle(-63, 94, 75); circle(63, 94, 75); path('M-44 64 L84 -133 L5 -5 L-84 -133 L44 64', { color: 'soft', sw: 5 }); circle(0, 2, 19);
        break;
      case 'braid':
        for (let i = 0; i < 7; i++) path(`M-27 ${-145 + i * 39} Q44 ${-129 + i * 39} -27 ${-106 + i * 39} M27 ${-145 + i * 39} Q-44 ${-129 + i * 39} 27 ${-106 + i * 39}`, { color: 'soft', sw: 5 });
        break;
      case 'racket':
        ellipse(-22, -57, 163, 210, { color: 'soft' }); line(6, 42, 68, 156, { color: 'orange', sw: 10 });
        for (let i = -2; i <= 2; i++) { line(-77, -58 + i * 25, 31, -58 + i * 25, { color: 'muted', sw: 1 }); line(-22 + i * 23, -137, -22 + i * 23, 22, { color: 'muted', sw: 1 }); }
        circle(115, -83, 50, { color: 'green' });
        break;
      case 'badge':
        path('M-44 -146 L0 -53 L44 -146 M-76 -51 H76 V141 H-76 Z', { fill: 'faint' }); circle(0, 7, 47, { color: 'soft' }); writing(-47, 57, 94, 2);
        break;
      case 'candle':
        box(-39, -19, 78, 155, { fill: 'yellowF', color: 'orange' }); path('M0 -143 Q83 -66 0 -35 Q-58 -69 0 -143 Z', { color: 'red', fill: 'redF' }); line(0, -20, 0, -48, { color: 'soft' }); ellipse(0, 141, 157, 29);
        break;
      case 'toy':
        circle(0, -45, 122, { fill: 'yellowF', color: 'orange' }); circle(-66, -101, 53, { color: 'orange' }); circle(66, -101, 53, { color: 'orange' }); ellipse(0, 75, 143, 170, { fill: 'faint' });
        break;
      default: throw new Error('Unknown episode object: ' + name);
    }
    return Q;
  }

  function draw(P) {
    const pg = P.page, span = pg.drawDuration;
    const times = pg.beats.map(b => b.at);
    P.text('读懂一本书', { x: 180, y: 77, size: 28, color: 'muted', at: -1, cps: 40, pen: false });
    const bookTitle = Array.from(SCRIPT.META.title);
    P.text(bookTitle.length > 11 ? bookTitle.slice(0, 10).join('') + '…' : SCRIPT.META.title, { x: 1740, y: 77, w: 340, align: 'r', size: 28, color: 'muted', at: -1, cps: 40, pen: false });
    P.text(pg.title, { x: 180, y: 160, size: 62, at: -.5, cps: 18 });
    P.line(180, 261, 1740, 261, { color: 'muted', sw: 1.2, at: -.3, dur: 1 });
    P.text(String(pg.sourceIndex + 1).padStart(2, '0'), { x: 1740, y: 817, w: 75, align: 'r', font: 'mono', size: 25, color: 'muted', at: -1, cps: 30, pen: false });
    let places = [{ x: 450, y: 511, s: 1.14 }, { x: 960, y: 511, s: .98 }, { x: 1470, y: 511, s: .98 }];
    if (pg.kind === 'compare') places = places.map(pos => ({ ...pos, s: 1.02 }));
    if (['opening', 'closing', 'desk', 'focus'].includes(pg.kind)) places = [{ x: 552, y: 518, s: 1.25 }, { x: 1132, y: 433, s: .8 }, { x: 1476, y: 600, s: .78 }];
    if (['route', 'escape'].includes(pg.kind)) {
      places = [{ x: 440, y: 574, s: 1.08 }, { x: 960, y: 501, s: 1 }, { x: 1470, y: 430, s: .96 }];
      P.path('M230 751 Q579 775 820 650 T1280 601 T1680 589', { color: 'green', at: -.3, dur: 3.2, sw: 2 });
    }
    if (pg.beats.length === 1) places = [{ x: 960, y: 485, s: 1.3 }];
    if (pg.beats.length === 2) places = pg.kind === 'compare'
      ? [{ x: 620, y: 500, s: 1.12 }, { x: 1300, y: 500, s: 1.12 }]
      : [{ x: 620, y: 500, s: 1.22 }, { x: 1320, y: 500, s: .98 }];
    if (pg.kind === 'wall') P.line(1212, 303, 1212, 736, { color: 'red', sw: 3, at: 1, dur: 2 });
    if (pg.kind === 'locked') P.path('M211 778 H1709', { color: 'red', sw: 2, at: -.3, dur: 2 });

    pg.beats.forEach((b, i) => {
      const pos = places[i], at = times[i];
      const Q = sketch(P, b.object, at, b.object === 'pen' ? 'orange' : 'ink');
      P.move(Q, t => {
        const u = K(t, at, 1.8);
        let dx = 0, dy = 0, rot = 0;
        if (b.action === 'slide') dx = -48 * (1 - u);
        if (b.action === 'tilt') { dy = 22 * u; rot = 12 * u; }
        if (b.action === 'sway') rot = 5 * Math.sin(Math.PI * K(t, at, 3));
        return { x: pos.x + dx, y: pos.y + dy, s: pos.s, rot };
      });
      const labelY = pg.beats.length < 3 ? 754 : ['opening', 'closing', 'desk', 'focus'].includes(pg.kind) ? (i === 0 ? 746 : pos.y + 148) : ['route', 'escape'].includes(pg.kind) ? 784 : 722;
      P.text(b.label, { x: pos.x, y: labelY, w: 408, align: 'c', size: 37, color: i === 0 ? 'ink' : 'soft', at, cps: 12 });
      if (i > 0 && ['route', 'transfer'].includes(pg.kind)) {
        const prev = places[i - 1];
        P.arrow(prev.x + 180, prev.y, pos.x - 180, pos.y, { color: 'green', at, dur: 1.2, sw: 2.5 });
      }
      if (b.action === 'check') Q.check(0, 0, 140, { color: 'green', at, dur: .9 });
      if (b.action === 'cross') Q.cross(0, 0, 140, { color: 'red', at, dur: .9 });
    });
    P.line(180, 873, 1630, 873, { color: 'faint', sw: 2, at: -1, dur: .1 });
    P.liveRough((t, rough) => [rough.line(180, 873, 180 + 1450 * Math.max(0, Math.min(1, t / span)), 873, { color: 'orange', sw: 2.5 })]);
  }
  window.EPISODE = { draw };
  const style = document.createElement('style');
  style.textContent = `.chrome{display:none!important}.subs{bottom:42px!important}.subs span{font-size:44px!important;line-height:1.35!important;color:var(--ink)!important;background:${PV.col('paper')}!important;border-radius:4px!important;box-shadow:none!important;padding:8px 28px!important}`;
  document.head.appendChild(style);
})();
