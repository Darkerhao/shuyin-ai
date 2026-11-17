/* Shared numeric compiler and Canvas renderer: editor, preview and film use one implementation. */
(function (root) {
    const fail = message => { throw new Error(message); };
    const number = (v, a, b) => typeof v === 'number' && Number.isFinite(v) && v >= a && v <= b;
    const keys = (o, names, optional = []) => {
        if (!o || typeof o !== 'object' || Array.isArray(o) || Object.keys(o).some(k => !names.includes(k) && !optional.includes(k)) || names.some(k => !Object.hasOwn(o, k)))
            fail('Invalid pencil fields.');
    };
    const text = (s, n) => typeof s === 'string' && s.trim() && [...s].length <= n && !/[\p{Cc}]/u.test(s);
    function path(commands, width = 1600, height = 900, closed = false) {
        if (!Array.isArray(commands) || commands.length < 2 || commands[0]?.length !== 2 || commands.some(c => !Array.isArray(c) || ![2, 4, 6].includes(c.length) || c.some((v, i) => !number(v, 0, i % 2 ? height : width))))
            fail('Invalid pencil coordinates.');
        if (closed && commands.at(-1).slice(-2).some((v, i) => v !== commands[0][i]))
            fail('Pencil region must close.');
        let a = commands[0], result = [a];
        for (const c of commands.slice(1)) {
            if (c.length === 2) {
                result.push(c);
                a = c;
                continue;
            }
            const b = c.slice(0, 2), d = c.slice(-2), n = Math.max(8, Math.min(60, Math.ceil((Math.hypot(d[0] - a[0], d[1] - a[1]) + Math.hypot(b[0] - a[0], b[1] - a[1])) / 5)));
            for (let i = 1; i <= n; i++) {
                const t = i / n, v = 1 - t;
                result.push(c.length === 4 ? [v * v * a[0] + 2 * v * t * b[0] + t * t * d[0], v * v * a[1] + 2 * v * t * b[1] + t * t * d[1]] : [v * v * v * a[0] + 3 * v * v * t * b[0] + 3 * v * t * t * c[2] + t * t * t * d[0], v * v * v * a[1] + 3 * v * v * t * b[1] + 3 * v * t * t * c[3] + t * t * t * d[1]]);
            }
            a = d;
        }
        if (!result.some(p => p.some((v, i) => v !== result[0][i])))
            fail('Zero length pencil path.');
        return result;
    }
    function geometry(g, width = 1600, height = 900) {
        if (![g.mask, g.strokes, g.shades].every(Array.isArray) || g.mask.length > 24 || g.strokes.length > 600 || g.shades.length > 60)
            fail('Pencil geometry budget exceeded.');
        let commands = 0;
        for (const p of g.mask) {
            path(p, width, height, true);
            commands += p.length;
        }
        for (const s of g.strokes) {
            keys(s, ['p', 'w', 'a']);
            if (!number(s.w, .4, 4.5) || !number(s.a, .05, .9))
                fail('Invalid pencil stroke.');
            path(s.p, width, height);
            commands += s.p.length;
        }
        for (const s of g.shades) {
            keys(s, ['p', 'w', 'a', 'angle', 'gap']);
            if (!number(s.w, .4, 4.5) || !number(s.a, .05, .9) || !number(s.angle, 0, 180) || !number(s.gap, 2.5, 12))
                fail('Invalid pencil shade.');
            path(s.p, width, height, true);
            commands += s.p.length;
        }
        return commands;
    }
    function hatch(shade, transform = p => p) {
        const poly = path(shade.p).map(transform), angle = shade.angle * Math.PI / 180, ux = Math.cos(angle), uy = Math.sin(angle), vx = -uy, vy = ux, p = poly.map(([x, y]) => [x * ux + y * uy, x * vx + y * vy]), low = Math.min(...p.map(p => p[1])), high = Math.max(...p.map(p => p[1])), lines = [];
        for (let v = low + shade.gap * .4; v < high; v += shade.gap) {
            const hits = [];
            for (let i = 0; i < p.length - 1; i++) {
                const a = p[i], b = p[i + 1];
                if ((a[1] <= v && b[1] > v) || (b[1] <= v && a[1] > v))
                    hits.push(a[0] + (v - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
            }
            hits.sort((a, b) => a - b);
            for (let i = 0; i + 1 < hits.length; i += 2) {
                const u = hits[i], end = hits[i + 1];
                if (end - u < .8)
                    continue;
                lines.push([[u * ux + v * vx, u * uy + v * vy], [end * ux + v * vx, end * uy + v * vy]]);
                if (lines.length > 6000)
                    return lines;
            }
        }
        return lines;
    }
    function validate(art, characters, first, last) {
        keys(art, ['design', 'layers']);
        if (!text(art.design, 6000) || !Array.isArray(art.layers) || !art.layers.length || art.layers.length > 24)
            fail('Invalid pencil scene.');
        if (!Array.isArray(characters) || characters.length > 64)
            fail('Invalid pencil cast.');
        const ids = new Set();
        for (const c of characters) {
            keys(c, ['id', 'name', 'description', 'heads']);
            if (!text(c.id, 64) || !text(c.name, 80) || !text(c.description, 1600) || !/^[A-Za-z0-9_]+$/.test(c.id) || ids.has(c.id))
                fail('Invalid pencil character.');
            ids.add(c.id);
            keys(c.heads, ['front', 'side']);
            for (const head of Object.values(c.heads)) {
                keys(head, ['mask', 'strokes', 'shades']);
                if (!head.strokes.length || head.strokes.length > 100 || geometry(head, 200, 240) > 1600)
                    fail('Invalid frozen head.');
            }
        }
        let previous = first, commands = 0, strokes = 0, shades = 0, texts = 0, expanded = 0;
        const seen = new Set();
        art.layers.forEach((l, i) => {
            keys(l, ['cue', 'name', 'mask', 'strokes', 'shades', 'texts'], ['portrait']);
            if (!Number.isInteger(l.cue) || l.cue < previous || l.cue > last || (i === 0 && l.cue !== first) || !text(l.name, 80))
                fail('Invalid pencil cue.');
            previous = l.cue;
            commands += geometry(l);
            strokes += l.strokes.length;
            shades += l.shades.length;
            if (!Array.isArray(l.texts))
                fail('Invalid pencil labels.');
            texts += l.texts.length;
            for (const t of l.texts) {
                keys(t, ['x', 'y', 'text', 'size', 'a']);
                if (!number(t.x, 0, 1600) || !number(t.y, 0, 900) || !text(t.text, 40) || !number(t.size, 16, 30) || !number(t.a, .15, .9))
                    fail('Invalid pencil label.');
            }
            expanded += l.strokes.length;
            for (const s of l.shades) {
                expanded += hatch(s).length;
                if (expanded > 6000)
                    fail('Expanded pencil resource budget exceeded.');
            }
            if (l.portrait != null) {
                const p = l.portrait;
                keys(p, ['characterId', 'view', 'x', 'y', 'scale', 'flip']);
                if (!ids.has(p.characterId) || seen.has(p.characterId) || !['front', 'side'].includes(p.view) || typeof p.flip !== 'boolean' || !number(p.scale, .15, 3) || !number(p.x, 0, 1600 - 200 * p.scale) || !number(p.y, 0, 900 - 240 * p.scale))
                    fail('Invalid frozen portrait placement.');
                seen.add(p.characterId);
                const head = characters.find(c => c.id === p.characterId).heads[p.view];
                expanded += head.strokes.length;
                for (const s of head.shades)
                    expanded += hatch(s, ([x, y]) => [(p.flip ? 200 - x : x) * p.scale, y * p.scale]).length;
            }
            if (expanded > 6000)
                fail('Expanded pencil resource budget exceeded.');
        });
        if (commands > 6500 || strokes > 600 || shades > 60 || texts > 30)
            fail('Pencil scene budget exceeded.');
    }
    function compile(art, characters, cues, start, duration, first, last) {
        validate(art, characters, first, last);
        if (!number(start, 0, 86400) || !number(duration, .001, 7200))
            fail('Invalid pencil clock.');
        const time = cue => { const c = cues.find(c => c.index === cue); if (!c || !number(c.start, start, start + duration))
            fail('Missing pencil cue clock.'); return c.start - start; };
        const layers = art.layers.map(l => ({ cue: l.cue, mask: [], strokes: [], texts: l.texts, at: time(l.cue) }));
        const append = (out, g, transform = p => p) => {
            out.mask.push(...g.mask.map(p => path(p).map(transform)));
            for (const s of g.strokes) {
                const points = path(s.p).map(transform);
                out.strokes.push({ points, w: s.w, a: s.a, primary: s.w >= 1.1, length: points.slice(1).reduce((n, p, i) => n + Math.hypot(p[0] - points[i][0], p[1] - points[i][1]), 0) });
            }
            for (const s of g.shades)
                for (const points of hatch(s, transform))
                    out.strokes.push({ points, w: s.w, a: s.a, primary: false, length: Math.hypot(points[1][0] - points[0][0], points[1][1] - points[0][1]) });
        };
        art.layers.forEach((l, i) => {
            const out = layers[i];
            append(out, l);
            if (l.portrait) {
                const p = l.portrait, head = characters.find(c => c.id === p.characterId).heads[p.view];
                // A separate sublayer masks earlier body/background before drawing the immutable head.
                const h = { cue: l.cue, mask: [], strokes: [], texts: [], at: time(l.cue), portrait: p.characterId, view: p.view };
                append(h, head, ([x, y]) => [p.x + (p.flip ? 200 - x : x) * p.scale, p.y + y * p.scale]);
                out.head = h;
            }
        });
        const flattened = layers.flatMap(l => l.head ? [l, l.head] : [l]);
        if (flattened.reduce((n, l) => n + l.strokes.length, 0) > 6000)
            fail('Expanded pencil resource budget exceeded.');
        const anchors = [...new Set(flattened.map(l => l.cue))];
        for (let i = 0; i < anchors.length; i++) {
            const anchor = anchors[i], at = time(anchor), end = i + 1 < anchors.length ? time(anchors[i + 1]) : duration, window = end - at;
            const group = flattened.filter(l => l.cue === anchor), all = group.flatMap(l => l.strokes), capacity = Math.floor(window * .72 / .08);
            const main = all.filter(s => s.primary).sort((a, b) => b.length - a.length).slice(0, capacity), selected = new Set(main);
            const ordered = all.filter(s => selected.has(s)), weight = ordered.reduce((n, s) => n + Math.sqrt(s.length) + 2, 0), remaining = Math.max(0, window * .72 - main.length * .08);
            let elapsed = at;
            for (const s of all) {
                s.primary = selected.has(s);
                if (s.primary) {
                    s.at = elapsed;
                    s.duration = .08 + remaining * (Math.sqrt(s.length) + 2) / (weight || 1);
                    elapsed += s.duration;
                }
                else {
                    s.at = at + window * .45;
                    s.duration = Math.max(.02, window * .45);
                }
            }
            for (const l of group) {
                l.at = at;
                l.textAt = at + window * .45;
            }
        }
        return { layers: flattened, duration, width: 1600, height: 900 };
    }
    function trace(ctx, points, fraction = 1) {
        ctx.beginPath();
        ctx.moveTo(...points[0]);
        let target = points.slice(1).reduce((n, p, i) => n + Math.hypot(p[0] - points[i][0], p[1] - points[i][1]), 0) * fraction, tip = points[0];
        for (let i = 1; i < points.length; i++) {
            const a = points[i - 1], b = points[i], length = Math.hypot(b[0] - a[0], b[1] - a[1]);
            if (target < length) {
                const u = target / (length || 1);
                tip = [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
                ctx.lineTo(...tip);
                break;
            }
            ctx.lineTo(...b);
            target -= length;
            tip = b;
        }
        return tip;
    }
    function render(canvas, compiled, time, pen = true) {
        const ctx = canvas.getContext('2d');
        if (!ctx)
            return;
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.setTransform(canvas.width / 1600, 0, 0, canvas.height / 900, 0, 0);
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
        let tip;
        for (const l of compiled.layers) {
            if (time < l.at)
                continue;
            ctx.fillStyle = '#fbfaf7';
            for (const p of l.mask) {
                trace(ctx, p);
                ctx.closePath();
                ctx.fill();
            }
            for (const s of l.strokes) {
                if (time < s.at)
                    continue;
                const progress = Math.min(1, (time - s.at) / s.duration);
                ctx.lineWidth = s.w;
                ctx.strokeStyle = `rgba(47,45,41,${s.a * (s.primary ? 1 : progress)})`;
                const point = trace(ctx, s.points, s.primary ? progress : 1);
                ctx.stroke();
                if (s.primary && progress < 1)
                    tip = point;
            }
            for (const t of l.texts) {
                if (time < l.textAt)
                    continue;
                ctx.fillStyle = `rgba(57,53,46,${t.a})`;
                ctx.font = `${t.size}px "Microsoft YaHei",sans-serif`;
                ctx.fillText(t.text, t.x, t.y);
            }
        }
        if (pen && tip) {
            ctx.save();
            ctx.translate(...tip);
            ctx.fillStyle = '#70726f';
            ctx.beginPath();
            ctx.moveTo(0, 0);
            ctx.lineTo(3, -13);
            ctx.lineTo(29, -62);
            ctx.lineTo(36, -57);
            ctx.lineTo(11, -9);
            ctx.closePath();
            ctx.fill();
            ctx.fillStyle = '#ccb89a';
            ctx.beginPath();
            ctx.moveTo(0, 0);
            ctx.lineTo(3, -13);
            ctx.lineTo(11, -9);
            ctx.closePath();
            ctx.fill();
            ctx.fillStyle = '#33322f';
            ctx.beginPath();
            ctx.moveTo(0, 0);
            ctx.lineTo(1, -5);
            ctx.lineTo(5, -3);
            ctx.closePath();
            ctx.fill();
            ctx.restore();
        }
    }
    function mount(P) {
        const page = P.page, canvas = document.createElement('canvas');
        canvas.width = 1920;
        canvas.height = 1080;
        canvas.style.cssText = 'position:absolute;inset:0;width:1920px;height:1080px;pointer-events:none';
        P.host.appendChild(canvas);
        const cues = page.pencilCues;
        const compiled = compile(page.artwork, page.characters ?? [], cues, 0, page.drawDuration, page.first, page.last);
        P.tick(t => render(canvas, compiled, t, true));
    }
    root.SHUYIN_PENCIL_DRAWING = { validate, compile, render, mount };
})(typeof window === 'undefined' ? globalThis : window);
