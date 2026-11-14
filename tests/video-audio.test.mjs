import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { muxVideoAudio } from '../src-tauri/video/render.mjs';

// Use the exact engine binary locally/CI; ordinary unit tests do not install FFmpeg.
const ffmpeg = process.env.SHUYIN_TEST_FFMPEG;
function command(args) {
  const result = spawnSync(ffmpeg, ['-hide_banner', '-nostdin', ...args], { encoding: 'utf8', windowsHide: true, timeout: 30000, maxBuffer: 4 * 1024 * 1024 });
  assert.equal(result.status, 0, result.error?.message || result.stderr);
  return result;
}
function measure(file, filter = '') {
  const result = command(['-i', file, '-map', '0:a:0', '-af', `${filter ? filter + ',' : ''}loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json`, '-f', 'null', '-']);
  return JSON.parse(/\{[^{}]*"input_i"[^{}]*\}/s.exec(result.stderr)[0]);
}
function fixture(t, signal, duration = 12) {
  const directory = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'shuyin-audio-')));
  assert.equal(path.dirname(directory), fs.realpathSync(os.tmpdir()));
  t.after(() => fs.rmSync(directory, { recursive: true }));
  const audio = path.join(directory, '原声 & source.wav'), video = path.join(directory, 'picture.mp4'), output = path.join(directory, 'result.mp4');
  command(['-f', 'lavfi', '-i', signal, '-t', String(duration), '-c:a', 'pcm_s16le', audio]);
  command(['-f', 'lavfi', '-i', 'color=c=blue:s=160x90:r=24', '-t', String(duration), '-an', '-c:v', 'libx264', '-preset', 'ultrafast', video]);
  return { audio, video, output, duration, directory };
}
function videoHash(file) {
  return command(['-v', 'error', '-i', file, '-map', '0:v:0', '-c:v', 'copy', '-f', 'hash', '-']).stdout.trim();
}

test('quiet source becomes audible in the actual video mux without changing the source or picture', { skip: !ffmpeg }, async t => {
  const input = fixture(t, 'aevalsrc=0.035*sin(2*PI*220*t)*(0.65+0.35*sin(2*PI*2*t)):s=44100');
  const digest = createHash('sha256').update(fs.readFileSync(input.audio)).digest('hex');
  const before = measure(input.audio), updates = [];
  await muxVideoAudio(ffmpeg, input, (phase, counts) => updates.push({ phase, ...counts }));
  const after = measure(input.output);
  assert(Number(before.input_i) < -28, JSON.stringify(before));
  assert(Math.abs(Number(after.input_i) + 16) <= 1, JSON.stringify(after));
  assert(Number(after.input_tp) <= -1.5, JSON.stringify(after));
  assert.equal(videoHash(input.video), videoHash(input.output));
  assert.equal(createHash('sha256').update(fs.readFileSync(input.audio)).digest('hex'), digest);
  assert(updates.some(p => p.phase === 'measuring_audio' && p.total === input.duration));
  assert(updates.some(p => p.phase === 'muxing_audio' && p.completed > 0));
  assert.equal(fs.readdirSync(input.directory).some(name => name.startsWith('normalized-audio-')), false);
});

test('preview uses the full audio dynamics before cutting its window and retains timing', { skip: !ffmpeg }, async t => {
  const input = fixture(t, 'aevalsrc=(0.045+0.6*lt(t\\,5)*lt(mod(t\\,2)\\,0.05))*sin(2*PI*330*t)*(0.7+0.3*sin(2*PI*3*t)):s=48000');
  await muxVideoAudio(ffmpeg, input, () => {});
  const preview = { ...input, output: path.join(input.directory, 'preview.mp4'), range: { start: 6, end: 10 } };
  const previewVideo = path.join(input.directory, 'preview-picture.mp4');
  command(['-i', input.video, '-t', '4', '-c:v', 'copy', previewVideo]);
  preview.video = previewVideo;
  const updates = [];
  await muxVideoAudio(ffmpeg, preview, (phase, counts) => updates.push({ phase, ...counts }));
  const fullSegment = measure(input.output, 'atrim=start=6:end=10,asetpts=PTS-STARTPTS');
  const segment = measure(preview.output);
  assert(Math.abs(Number(segment.input_i) - Number(fullSegment.input_i)) < 0.3, JSON.stringify({ fullSegment, segment }));
  const duration = command(['-v', 'error', '-i', preview.output, '-map', '0:a:0', '-progress', 'pipe:1', '-f', 'null', '-']).stdout;
  assert(Math.abs(Number([...duration.matchAll(/out_time_us=(\d+)/g)].at(-1)[1]) / 1e6 - 4) < 0.06, duration);
  assert(updates.some(p => p.phase === 'measuring_audio' && p.total === input.duration));
});

test('loud transients stay below the delivery ceiling after AAC encoding', { skip: !ffmpeg }, async t => {
  const input = fixture(t, 'aevalsrc=(0.13+0.75*lt(mod(t\\,2)\\,0.05))*sin(2*PI*880*t):s=48000');
  const updates = [];
  await muxVideoAudio(ffmpeg, input, (phase, counts) => updates.push({ phase, ...counts }));
  const after = measure(input.output);
  assert(Math.abs(Number(after.input_i) + 16) <= 1, JSON.stringify(after));
  assert(Number(after.input_tp) <= -1.5, JSON.stringify(after));
  const attempts = updates.filter((p, index) => p.phase === 'normalizing_audio' && updates[index - 1]?.phase !== p.phase);
  assert(attempts.length >= 1 && attempts.length <= 2, 'Peak correction is bounded.');
});

test('digital silence stays silent and measurement errors cannot produce a successful video', { skip: !ffmpeg }, async t => {
  const input = fixture(t, 'anullsrc=r=48000:cl=stereo', 2);
  await muxVideoAudio(ffmpeg, input, () => {});
  assert.equal(measure(input.output).input_i, '-inf');
  const output = path.join(input.directory, 'failed.mp4');
  await assert.rejects(muxVideoAudio(ffmpeg, { ...input, audio: path.join(input.directory, 'missing.wav'), output }, () => {}), /无法测量原音频响度/);
  assert.equal(fs.existsSync(output), false);
  const saved = fs.readFileSync(input.output);
  await assert.rejects(muxVideoAudio(ffmpeg, input, () => {}), /输出视频已存在/);
  assert.deepEqual(fs.readFileSync(input.output), saved, 'An existing result must never be overwritten.');
  assert.equal(fs.readdirSync(input.directory).some(name => name.startsWith('normalized-audio-')), false);
});

test('a missing picture fails after audio preparation and cleans the temporary track', { skip: !ffmpeg }, async t => {
  const input = fixture(t, 'anullsrc=r=48000:cl=stereo', 2), phases = [];
  const original = fs.readFileSync(input.audio);
  await assert.rejects(muxVideoAudio(ffmpeg, { ...input, video: path.join(input.directory, 'missing.mp4') }, phase => phases.push(phase)), /Render process exited/);
  assert(phases.includes('checking_audio'));
  assert(phases.includes('muxing_audio'));
  assert.equal(fs.existsSync(input.output), false);
  assert.deepEqual(fs.readFileSync(input.audio), original);
  assert.equal(fs.readdirSync(input.directory).some(name => name.startsWith('normalized-audio-')), false);
});

test('Windows write denial fails without changing the source or creating a result', { skip: !ffmpeg || process.platform !== 'win32' }, async t => {
  const input = fixture(t, 'anullsrc=r=48000:cl=stereo', 2);
  const original = fs.readFileSync(input.audio);
  const access = args => {
    const result = spawnSync('icacls.exe', [input.directory, ...args], { encoding: 'utf8', windowsHide: true, timeout: 10000 });
    assert.equal(result.status, 0, result.error?.message || result.stderr || result.stdout);
  };
  try {
    // This explicit ACL applies only to the newly created test directory.
    access(['/deny', '*S-1-1-0:(W)']);
    await assert.rejects(muxVideoAudio(ffmpeg, input, () => {}), error => error.syscall === 'mkdtemp' && ['EACCES', 'EPERM'].includes(error.code));
  } finally {
    access(['/remove:d', '*S-1-1-0']);
  }
  assert.equal(fs.existsSync(input.output), false);
  assert.deepEqual(fs.readFileSync(input.audio), original);
  assert.equal(fs.readdirSync(input.directory).some(name => name.startsWith('normalized-audio-')), false);
});
