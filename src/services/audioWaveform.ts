const cached = new Map<string, Promise<number[]>>()
let decoding = Promise.resolve()

export function audioPeaks(buffer: Pick<AudioBuffer, 'numberOfChannels' | 'length' | 'getChannelData'>, count = 256) {
  const peaks = Array<number>(count).fill(0)
  for (let channel = 0; channel < buffer.numberOfChannels; channel++) {
    const samples = buffer.getChannelData(channel)
    for (let bar = 0; bar < count; bar++) {
      const start = Math.floor(bar * samples.length / count)
      const end = Math.floor((bar + 1) * samples.length / count)
      let squares = 0; let size = 0
      for (let sample = start; sample < end; sample += 4) { squares += samples[sample]! ** 2; size++ }
      peaks[bar] = Math.max(peaks[bar]!, size ? Math.sqrt(squares / size) : 0)
    }
  }
  const maximum = Math.max(...peaks)
  return maximum ? peaks.map(peak => peak / maximum) : peaks
}

export function loadWaveform(source: string) {
  const existing = cached.get(source)
  if (existing) return existing
  // Decode one local file at a time, at 8 kHz, then retain only its small amplitude envelope.
  const result = decoding.then(async () => {
    const response = await fetch(source)
    if (!response.ok) throw new Error('音频波形无法读取。')
    const buffer = await new OfflineAudioContext(1, 1, 8000).decodeAudioData(await response.arrayBuffer())
    return audioPeaks(buffer)
  })
  decoding = result.then(() => {}, () => {})
  cached.set(source, result)
  if (cached.size > 4) cached.delete(cached.keys().next().value!)
  void result.catch(() => { if (cached.get(source) === result) cached.delete(source) })
  return result
}
