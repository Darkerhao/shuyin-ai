import type { PencilArtwork, PencilCharacter, VideoCue } from '../../src/types/video'
export interface PencilCompiled { duration: number; width: number; height: number; layers: unknown[] }
declare global {
  var SHUYIN_PENCIL_DRAWING: {
    validate(art: PencilArtwork, characters: PencilCharacter[], first: number, last: number): void
    compile(art: PencilArtwork, characters: PencilCharacter[], cues: (VideoCue & { index: number })[], start: number, duration: number, first: number, last: number): PencilCompiled
    render(canvas: HTMLCanvasElement, compiled: PencilCompiled, time: number, pen?: boolean): void
  }
}
export {}
