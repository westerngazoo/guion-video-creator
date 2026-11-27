"""An external voice for guion (R-0020): Qwen3-TTS, cloned from one reference clip.

    voz_qwen.py --modelo DIR --referencia WAV --texto-referencia TXT \\
                --semilla N --silencio S [--idioma Spanish] [--intentos 4] SALIDA.wav

Reads the utterance's sentences on stdin, one per line (guion splits them),
speaks them as ONE utterance with the voice of the reference clip (Qwen3-TTS
Base, in-context cloning: the clip and its transcript), and writes SALIDA.wav:
mono 16-bit PCM in which every two sentences are separated by a run of exact
zeros at least `--silencio` s long and no other sample is zero
(`alinear.a_ceros`), the `Voice` contract guion cuts takes by.

Sampling is random: the take is kept by guion (`voz.tomas/`), never spoken
again, so the reel stays byte-deterministic. A take that cannot be cut into
its sentences (`TomaRechazada`) is spoken again with the next seed, up to
`--intentos` times; then the script fails with exit status 1 and writes nothing.

Runs on Apple's MPS when present (bf16), else on the CPU (fp32).
Model: Qwen3-TTS-12Hz-0.6B-Base or -1.7B-Base (Apache-2.0),
https://huggingface.co/Qwen/Qwen3-TTS-12Hz-0.6B-Base
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
import torch
from qwen_tts import Qwen3TTSModel

sys.path.insert(0, str(Path(__file__).resolve().parent))
from alinear import TomaRechazada, a_ceros, marcas  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--modelo", required=True)
    ap.add_argument("--referencia", required=True)
    ap.add_argument("--texto-referencia", required=True)
    ap.add_argument("--semilla", type=int, required=True)
    ap.add_argument("--silencio", type=float, required=True)
    ap.add_argument("--idioma", default="Spanish")
    ap.add_argument("--intentos", type=int, default=4)
    ap.add_argument("salida")
    a = ap.parse_args()

    frases = [l.strip() for l in sys.stdin.read().splitlines() if l.strip()]
    if not frases:
        print("voz_qwen: no sentences on stdin", file=sys.stderr)
        return 1
    try:
        marcas(frases)  # before loading 2.5 GB: a sentence without letters cannot be cut
    except ValueError as e:
        print(f"voz_qwen: {e}: {frases}", file=sys.stderr)
        return 1
    texto_ref = Path(a.texto_referencia).read_text(encoding="utf-8").strip()

    mps = torch.backends.mps.is_available()
    modelo = Qwen3TTSModel.from_pretrained(
        a.modelo,
        device_map="mps" if mps else "cpu",
        dtype=torch.bfloat16 if mps else torch.float32,
        attn_implementation="sdpa",
    )
    prompt = modelo.create_voice_clone_prompt(ref_audio=a.referencia, ref_text=texto_ref)
    texto = " ".join(frases)
    for intento in range(a.intentos):
        semilla = a.semilla + intento
        torch.manual_seed(semilla)
        wavs, sr = modelo.generate_voice_clone(
            text=texto, language=a.idioma, voice_clone_prompt=prompt
        )
        x = np.asarray(wavs[0], dtype=np.float32)
        try:
            toma = a_ceros(x, sr, frases, a.silencio)
        except TomaRechazada as e:
            print(f"voz_qwen: seed {semilla} rejected: {e}", file=sys.stderr)
            continue
        sf.write(a.salida, toma, sr, subtype="PCM_16")
        print(f"voz_qwen: seed {semilla}, {len(toma) / sr:.2f} s", file=sys.stderr)
        return 0
    print(f"voz_qwen: {a.intentos} takes rejected", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
