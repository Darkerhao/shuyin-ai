# Reading-film renderer

`illustrations.js` adapts the trusted reading-film drawings actually used by
`whiteboard-teacher-video`, verified against the local editable film
`D:/films/视频风格实验/09-我的前半生-溥仪/extras/visuals.js` on 2026-11-01.
It is not a copy of Frametell engine code. The external AGPL engine remains
independently installed and is not bundled with the app.

The original 85 object drawings are unchanged. Removing only the appended P1
cases from the normalized `sketch` function preserves SHA-256
`96a6bc4e09c4440682d68e97f417b1357cf38c7a0a327482084d8f590b36fe9b`.
The source-audio clock, handwritten labels, progress stroke, and subtitle style
are retained. P0 centers a lone subject, gives narrative scenes a visual lead,
and keeps comparisons balanced. Objects and explicit semantic actions now start
at their selected subtitle, without pre-drawing the first object or adding a
fixed action delay. This no longer promises pixel parity with the old layout.

P1 adds eight objects (brain, compass, road_fork, mountain, tree, window, cup,
bed), six light environments plus plain, and wide/medium/detail framing plus
reference. All use the same primitives, catalog, and draw function. Backgrounds
contain spatial structure only; narrative props still count towards three beats.
Old object plans default to plain/reference without rewriting stored JSON.
The reference layout keeps P0 geometry; new framings reserve space for labels
and rotated objects. Only focus/detail with at least three seconds after the
first cue gets one 1.06x, three-second camera move. Titles and the progress
stroke use the engine HUD, while subtitles stay on its independent fixed layer.

`visual-catalog.json` is the single source of allowed objects, scenes, actions,
and limits for Rust validation, the model prompt, Node compilation, and Vue.
Model output is JSON data only. Old text-card plans remain readable but cannot
render through a second legacy template. Users explicitly regenerate the
storyboard from saved subtitles, or create a separate version of a completed film.

All three runtime files are embedded by Rust and written to the task directory.
The render cache key includes the compiler, drawing source, catalog, and plan.
Optional preview windows share this compiler and renderer. They retain source
cue IDs, scene numbers, object times, and full drawing durations, while clipping
only the viewing/audio window. Preview results and up to 12 renderer screenshots
are derived cache files, not registered video assets. The host validates the
current run and plan/source signature before exposing individual output files.
Run `npm run test:video` for timing, reference geometry, the complete catalog,
action determinism, scene/framing validation, camera/preview parity, old-plan
rejection, and rendering lifecycle checks. Current implementation and runtime
evidence are documented in `docs/VIDEO-SCENES-VERIFICATION.md`.
