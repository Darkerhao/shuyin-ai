"""Local-only transcription adapter. The host owns task state and validates output."""
import json
import sys
import time
from importlib.metadata import version
from pathlib import Path


def transcribe(source, model_path, context=""):
    if version("faster-whisper") != "1.2.1" or version("av") != "16.1.0":
        raise RuntimeError("Install the tested faster-whisper==1.2.1 and av==16.1.0 runtime first.")
    required = ("model.bin", "config.json", "tokenizer.json", "vocabulary.txt")
    if any(not (model_path / name).is_file() for name in required):
        raise RuntimeError("The local model is incomplete. Download all model files before transcription.")
    from faster_whisper import WhisperModel

    model = WhisperModel(str(model_path), device="cpu", compute_type="int8",
                         cpu_threads=4, local_files_only=True)
    segments, info = model.transcribe(str(source), language="zh", beam_size=5,
                                      word_timestamps=True, vad_filter=True,
                                      hotwords=context[:120] or None)
    cues = []
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
    if not cues:
        raise RuntimeError("No speech recognized. Check the audio or import a verified SRT.")
    return {"duration": info.duration, "cues": cues}


if __name__ == "__main__":
    try:
        if len(sys.argv) > 3:
            deadline = time.monotonic() + 10
            while not Path(sys.argv[3]).exists():
                if time.monotonic() > deadline:
                    raise RuntimeError("Host process gate timed out.")
                time.sleep(0.05)
        request = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
        result = transcribe(Path(request["audioPath"]), Path(request["modelPath"]), request.get("context", ""))
        with Path(sys.argv[2]).open("x", encoding="utf-8") as output:
            json.dump(result, output, ensure_ascii=False)
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
