param([Parameter(Mandatory = $true)][string]$Directory)
$ErrorActionPreference = 'Stop'
if (-not [System.IO.Path]::IsPathRooted($Directory)) { throw 'Use an absolute installation directory, preferably on D:.' }
$root = [System.IO.Path]::GetFullPath($Directory)
New-Item -ItemType Directory -Force -Path $root | Out-Null
$env:PIP_CACHE_DIR = Join-Path $root 'cache\pip'
$env:HF_HOME = Join-Path $root 'cache\huggingface'
$env:PYTHONUTF8 = '1'
$env:TEMP = Join-Path $root 'temp'
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
$python = Join-Path $root 'runtime\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $python)) {
    python -m venv (Join-Path $root 'runtime')
    if ($LASTEXITCODE -ne 0) { throw 'Python 3.12 is required to create the transcription environment.' }
}
& $python -m pip install -r (Join-Path $PSScriptRoot 'video-requirements.txt')
if ($LASTEXITCODE -ne 0) { throw 'Could not install the transcription dependencies.' }
$model = Join-Path $root 'model-small'
& $python -c 'import sys; from faster_whisper.utils import download_model; print(download_model("small", output_dir=sys.argv[1], revision="536b0662742c02347bc0e980a01041f333bce120"))' $model
if ($LASTEXITCODE -ne 0) { throw 'Model download failed. The existing audio and application data were not changed.' }
Write-Output "Python: $python"
Write-Output "Model: $model"
Write-Output 'Select these paths in Video production. Install/select Frametell 0.7.3 and Node separately.'
