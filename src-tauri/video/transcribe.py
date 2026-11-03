"""Local-only transcription adapter. The host owns task state and validates output."""
import json
import sys
import time
from importlib.metadata import version
from pathlib import Path


def check_python():
    if version("faster-whisper") != "1.2.1" or version("av") != "16.1.0":
        raise RuntimeError("Install the tested faster-whisper==1.2.1 and av==16.1.0 runtime first.")


def check_model(model_path):
    required = ("model.bin", "config.json", "tokenizer.json", "vocabulary.txt")
    if any(not (model_path / name).is_file() for name in required):
        raise RuntimeError("The local model is incomplete. Download all model files before transcription.")
    return model_path


def inspect_environment(model_path):
    items = []
    def python_runtime():
        check_python()
        __import__("faster_whisper")

    def load_model():
        check_model(model_path)
        from faster_whisper import WhisperModel
        WhisperModel(str(model_path), device="cpu", compute_type="int8", cpu_threads=1, local_files_only=True)

    for name, action in (("python", python_runtime), ("model", load_model)):
        try:
            if name == "model" and items[0]["status"] != "ok":
                raise RuntimeError("请先修复 Python 语音依赖，再检查模型。")
            action()
            items.append({"id": name, "status": "ok", "version": sys.version.split()[0] if name == "python" else "",
                          "path": sys.executable if name == "python" else str(model_path),
                          "detail": "faster-whisper 1.2.1 / av 16.1.0 可导入。" if name == "python" else "本地模型已离线加载（CPU / int8），未执行识别。"})
        except Exception as error:
            items.append({"id": name, "status": "error", "version": "", "path": "", "detail": str(error)})
    return items


def transcribe(source, model_path, context="", progress=None):
    check_python()
    check_model(model_path)
    from faster_whisper import WhisperModel

    def report(phase, completed=0, total=0, cues=()):
        if progress:
            progress({"stage": "transcribing", "phase": phase,
                      "completed": completed, "total": total,
                      "cueCount": len(cues), "preview": list(cues[-3:])})

    report("loading_model")
    model = WhisperModel(str(model_path), device="cpu", compute_type="int8",
                         cpu_threads=4, local_files_only=True)
    report("decoding_audio")
    segments, info = model.transcribe(str(source), language="zh", beam_size=5,
                                      word_timestamps=True, vad_filter=True,
                                      hotwords=context[:120] or None)
    cues = []
    completed = 0
    report("recognizing", total=info.duration)
    for segment in segments:
        words = segment.words or []
        text, start, end = "", None, None
        for word in words:
            # Use measured word boundaries; never distribute a sentence evenly over time.
            if text and (len(text + word.word) > 26 or word.end - start > 6):
                cues.append({"start": start, "end": end, "text": text.strip()})
                text, start = "", None
            start = word.start if start is None else start
            end = word.end
            text += word.word
            if text and text[-1] in "。！？!?；;":
                cues.append({"start": start, "end": end, "text": text.strip()})
                text, start = "", None
        if text.strip():
            cues.append({"start": start, "end": end, "text": text.strip()})
        completed = min(info.duration, max(completed, segment.end))
        report("recognizing", completed, info.duration, cues)
    if not cues:
        raise RuntimeError("No speech recognized. Check the audio or import a verified SRT.")
    report("finalizing_subtitles", info.duration, info.duration, cues)
    return {"duration": info.duration, "cues": cues}


def write_progress(path, progress):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(progress, ensure_ascii=False), encoding="utf-8")
    temporary.replace(path)


if __name__ == "__main__":
    try:
        if len(sys.argv) > 3:
            deadline = time.monotonic() + 10
            while not Path(sys.argv[3]).exists():
                if time.monotonic() > deadline:
                    raise RuntimeError("Host process gate timed out.")
                time.sleep(0.05)
        request = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
        progress_path = Path(sys.argv[2]).parent / "progress.json"
        result = inspect_environment(Path(request["modelPath"])) if request.get("action") == "environment" else transcribe(
            Path(request["audioPath"]), Path(request["modelPath"]), request.get("context", ""), lambda value: write_progress(progress_path, value))
        with Path(sys.argv[2]).open("x", encoding="utf-8") as output:
            json.dump(result, output, ensure_ascii=False)
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
