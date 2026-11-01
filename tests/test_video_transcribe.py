import importlib.util
import sys
import types
import unittest
import tempfile
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("video_transcribe", Path(__file__).parents[1] / "src-tauri/video/transcribe.py")
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


class TranscriptionTests(unittest.TestCase):
    def test_adapter_preserves_word_times_and_only_loads_local_model(self):
        calls = {}

        class Model:
            def __init__(self, path, **kwargs):
                calls.update(kwargs)

            def transcribe(self, source, **kwargs):
                calls.update(kwargs)
                word = lambda text, start, end: types.SimpleNamespace(word=text, start=start, end=end)
                segments = [types.SimpleNamespace(words=[word("第一句。", 0.5, 2), word("第二句。", 5, 8)])]
                return iter(segments), types.SimpleNamespace(duration=10)

        module = types.ModuleType("faster_whisper")
        module.WhisperModel = Model
        with tempfile.TemporaryDirectory() as directory:
            model = Path(directory)
            for name in ("model.bin", "config.json", "tokenizer.json", "vocabulary.txt"):
                (model / name).write_text("fixture", encoding="utf-8")
            with patch.object(adapter, "version", side_effect=lambda name: "16.1.0" if name == "av" else "1.2.1"), patch.dict(sys.modules, {"faster_whisper": module}):
                result = adapter.transcribe(Path("source.mp3"), model, "书名")
        self.assertEqual(result["cues"], [{"start": 0.5, "end": 2, "text": "第一句。"}, {"start": 5, "end": 8, "text": "第二句。"}])
        self.assertTrue(calls["local_files_only"])
        self.assertTrue(calls["word_timestamps"])
        self.assertEqual(calls["device"], "cpu")
        self.assertEqual(calls["hotwords"], "书名")

    def test_incompatible_runtime_fails_before_processing_audio(self):
        with patch.object(adapter, "version", return_value="19.0.2"):
            with self.assertRaisesRegex(RuntimeError, "av==16.1.0"):
                adapter.transcribe(Path("source.mp3"), Path("model"))

    def test_missing_local_tokenizer_does_not_attempt_a_download(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(adapter, "version", side_effect=lambda name: "16.1.0" if name == "av" else "1.2.1"):
            with self.assertRaisesRegex(RuntimeError, "incomplete"):
                adapter.transcribe(Path("source.mp3"), Path(directory))


if __name__ == "__main__":
    unittest.main()
