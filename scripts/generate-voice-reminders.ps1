param([string]$Voice = 'Microsoft Huihui Desktop')

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$voiceDirectory = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../src/assets/voice'))
$phrases = Get-Content -LiteralPath (Join-Path $voiceDirectory 'phrases.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$speaker = [System.Speech.Synthesis.SpeechSynthesizer]::new()
try {
    $speaker.SelectVoice($Voice)
    $format = [System.Speech.AudioFormat.SpeechAudioFormatInfo]::new(22050, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
    foreach ($phrase in $phrases.PSObject.Properties) {
        $speaker.SetOutputToWaveFile((Join-Path $voiceDirectory ($phrase.Name + '.wav')), $format)
        $speaker.Speak([string]$phrase.Value)
        $speaker.SetOutputToNull()
    }
} finally {
    $speaker.Dispose()
}
