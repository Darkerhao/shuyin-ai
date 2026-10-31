import { describe, expect, it } from 'vitest'
import { displayPath } from '../src/lib/displayPath'

describe('display paths', () => {
  it.each([
    [String.raw`\\?\D:\Books\My Book.epub`, String.raw`D:\Books\My Book.epub`],
    [String.raw`\\?\c:\books\notes.txt`, String.raw`c:\books\notes.txt`],
    [String.raw`\\?\UNC\server\share\audio.m4a`, String.raw`\\server\share\audio.m4a`],
    [String.raw`\\?\unc\server\share\audio.m4a`, String.raw`\\server\share\audio.m4a`],
    [String.raw`D:\Books\file.epub`, String.raw`D:\Books\file.epub`],
    [String.raw`\\server\share\file.epub`, String.raw`\\server\share\file.epub`],
    ['/home/books/file.epub', '/home/books/file.epub'],
    ['https://example.com/book?q=1', 'https://example.com/book?q=1'],
    [String.raw`\\?\Volume{example}\file`, String.raw`\\?\Volume{example}\file`],
    [String.raw`\\.\PhysicalDrive0`, String.raw`\\.\PhysicalDrive0`],
    ['', ''],
  ])('formats %s without rewriting path content', (input, expected) => {
    expect(displayPath(input)).toBe(expected)
    expect(displayPath(expected)).toBe(expected)
  })
})
