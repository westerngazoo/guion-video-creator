"""Tests of alinear.py on synthetic takes (numpy only, no model):

    python3 -I -m unittest discover -s tools/voz-qwen
"""

import unittest

import numpy as np

import alinear as A

SR = 24_000
RITMO = 14.0  # letters per second, about the cloned voice's


def toma(trozos: list[tuple[str, float]]) -> np.ndarray:
    """Speech for each piece of text (a tone, its letters at RITMO), each followed by its pause."""
    out = []
    for texto, pausa in trozos:
        n = int(A.letras(texto) / RITMO * SR)
        t = np.arange(n) / SR
        out.append(0.5 * np.sin(2 * np.pi * 220 * t) * (1 + 0.3 * np.sin(2 * np.pi * 3 * t)))
        out.append(np.full(int(pausa * SR), 1e-5))
    return np.concatenate(out).astype(np.float32)


FRASES = [
    "Por eso le das manos: tu agenda, tu correo.",
    "Le pides una cita.",
    "Y entonces te responde: cita agendada.",
]


class Alinear(unittest.TestCase):
    def test_a_comma_pause_longer_than_a_sentence_pause_is_not_a_cut(self):
        # as measured on the first Qwen take: the colon pauses 0.6 s, a period 0.45 s
        x = toma([
            ("Por eso le das manos:", 0.60), ("tu agenda,", 0.25), ("tu correo.", 0.45),
            ("Le pides una cita.", 0.50),
            ("Y entonces te responde:", 0.62), ("cita agendada.", 0.0),
        ])
        cortes = A.cortes(x, SR, FRASES)
        fin_1 = sum(A.letras(t) for t in ["Por eso le das manos:", "tu agenda,", "tu correo."]) / RITMO + 0.85
        self.assertEqual(len(cortes), 2)
        self.assertAlmostEqual(cortes[0][0] / SR, fin_1, delta=0.03)

    def test_the_cuts_become_exact_zeros_and_nothing_else_is_zero(self):
        x = toma([("Por eso le das manos:", 0.6), ("tu agenda, tu correo.", 0.5),
                  ("Le pides una cita.", 0.2), ("Y entonces te responde: cita agendada.", 0.0)])
        y = A.a_ceros(x, SR, FRASES, 0.3)
        z = np.flatnonzero(np.diff(np.r_[0, (y == 0).astype(int), 0]))
        runs = list(zip(z[::2], z[1::2]))
        self.assertEqual(len(runs), 2, "one zero run per cut, none elsewhere")
        self.assertGreaterEqual(min(b - a for a, b in runs), int(0.3 * SR), "a short pause is padded")
        self.assertEqual(y.dtype, np.int16)

    def test_a_take_with_a_missing_sentence_pause_is_rejected(self):
        x = toma([("Por eso le das manos: tu agenda, tu correo. Le pides una cita.", 0.5),
                  ("Y entonces te responde: cita agendada.", 0.0)])
        with self.assertRaises(A.TomaRechazada):
            A.cortes(x, SR, FRASES)

    def test_the_marks_follow_the_punctuation(self):
        ms = A.marcas(["Uno, dos.", "Tres."])
        self.assertEqual([m.fin for m in ms], [False, True])
        self.assertAlmostEqual(ms[0].lugar, 3 / 10)
        self.assertAlmostEqual(ms[1].lugar, 6 / 10)
        with self.assertRaises(ValueError):
            A.marcas(["…", "Hola."])


if __name__ == "__main__":
    unittest.main()
