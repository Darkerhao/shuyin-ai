# Reading-film renderer

`illustrations.js` adapts the trusted reading-film drawings actually used by
`whiteboard-teacher-video`, verified against the local editable film
`D:/films/视频风格实验/09-我的前半生-溥仪/extras/visuals.js` on 2026-11-01.
It is not a copy of Frametell engine code. The external AGPL engine remains
independently installed and is not bundled with the app.

The 85 object drawings are unchanged. The normalized `sketch` function has SHA-256
`96a6bc4e09c4440682d68e97f417b1357cf38c7a0a327482084d8f590b36fe9b`.
The reference composition, source-audio clock, handwritten labels, emphasis,
progress stroke, and subtitle styling are retained. Only book/series headers are
generalized; six bounded per-object actions extend the same movement code.

`visual-catalog.json` is the single source of allowed objects, scenes, actions,
and limits for Rust validation, the model prompt, Node compilation, and Vue.
Model output is JSON data only. Old text-card plans remain readable but cannot
render through a second legacy template. Users explicitly regenerate the
storyboard from saved subtitles, or create a separate version of a completed film.

All three runtime files are embedded by Rust and written to the task directory.
The render cache key includes the compiler, drawing source, catalog, and plan.
Run `npm run test:video` for timing, reference geometry, the complete catalog,
action determinism, old-plan rejection, and rendering lifecycle checks.
