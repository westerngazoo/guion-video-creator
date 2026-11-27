"""Find a take's sentence cuts and write them as exact zeros (guion R-0020).

Pure logic: numpy in, numpy out, no model and no files, so it tests without
mocks (test_alinear.py). `voz_qwen.py` is the I/O around it.

Why an alignment. guion cuts a take into sentences where it finds runs of
exact zeros (R-0019, `cut_take`): Piper writes them, a neural voice such as
Qwen3-TTS does not. Its pauses are quiet, not zero, and it pauses after a
comma or a colon about as long as between two sentences (measured on the
first take: 0.22–0.62 s after a comma, 0.48–0.84 s after a period), so "the
N longest pauses" are not the sentence cuts.

The text says where a pause may fall: after , ; : the voice may pause, after
. ? ! it must. Each mark has an expected place, its share of the letters said
before it; each quiet run has a measured place, its share of the speech heard
before it. A monotone alignment by dynamic programming (the recurrence of
dynamic time warping, Sakoe and Chiba 1978) pairs every sentence end with one
run, lets a comma pair with one run or none, and leaves the other runs
(breaths, stops) unpaired. A sentence spoken at a rate far from the take's
own means a wrong cut or a voice that skipped or repeated words: the take is
rejected and the caller speaks it again.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

# A pause is a run of 10 ms frames at least this far below the take's loudest frame.
UMBRAL_DB = -30.0
HOP_S = 0.01
# Shorter quiet runs are not pauses (a stop consonant); closer ones merge (a breath).
PAUSA_MIN_S = 0.10
UNIR_S = 0.05
# The alignment's costs (on the first takes, the right cut wins by a wide margin).
SIGMA = 0.04  # spread of a mark's measured place around its expected one
PAUSA_LIBRE_S = 0.30  # an unpaired pause this long costs 1
SALTO_COMA = 1.0  # a comma said without a pause
FIN_MIN_S = 0.25  # a sentence end in a shorter pause pays for it
# The checks after aligning.
ERROR_MAX = 0.08  # a cut this far from its sentence end: rejected (see ENSANCHE_MAX)
RITMO = (0.5, 2.0)  # a sentence's letters per second over the take's, outside: rejected
RITMO_LETRAS = 6  # …unless it is this short («Sí.»): one-syllable answers are drawn out
ENSANCHE_MAX = 0.12  # the most a very short neighbour widens ERROR_MAX to
# The edges of a zero run fade over this long (no click).
FUNDIDO_S = 0.015

COMAS = ",;:"
FINES = ".?!"


class TomaRechazada(Exception):
    """The take cannot be cut into its sentences; speak it again."""


@dataclass(frozen=True)
class Marca:
    """A place in the text where the voice may (coma) or must (fin) pause."""

    lugar: float  # share of the take's letters said before it, 0..1
    fin: bool


def letras(texto: str) -> int:
    return sum(c.isalnum() for c in texto)


def marcas(frases: list[str]) -> list[Marca]:
    """Every pause place in `frases`, in order, without the last sentence's end.

    Preconditions: no sentence is empty of letters, and numbers come written
    out («mil quinientos», the house rule for voz.toml), since a digit counts as
    one letter however long it is said. Each sentence's own end is
    a `fin` mark; a , ; or : inside it, followed by more letters, is a coma mark.
    """
    total = sum(letras(f) for f in frases)
    if total == 0 or any(letras(f) == 0 for f in frases):
        raise ValueError("una frase sin letras")
    out: list[Marca] = []
    antes = 0
    for i, f in enumerate(frases):
        dentro = 0
        for j, c in enumerate(f):
            if c.isalnum():
                dentro += 1
            elif c in COMAS and letras(f[j:]) > 0:
                out.append(Marca((antes + dentro) / total, False))
        antes += dentro
        if i + 1 < len(frases):
            out.append(Marca(antes / total, True))
    return out


def pausas(x: np.ndarray, sr: int) -> tuple[list[tuple[int, int]], int, int]:
    """The take's interior pauses as (start, end) samples, plus where speech starts and ends.

    Precondition: `x` is mono float and not all silent.
    """
    hop = max(1, int(sr * HOP_S))
    n = len(x) // hop
    if n == 0:
        raise TomaRechazada("la toma está vacía")
    rms = np.sqrt((x[: n * hop].reshape(n, hop).astype(np.float64) ** 2).mean(1) + 1e-12)
    callado = 20 * np.log10(rms / rms.max()) < UMBRAL_DB
    habla = np.flatnonzero(~callado)
    inicio, fin = int(habla[0]), int(habla[-1]) + 1
    runs: list[list[int]] = []
    k = inicio
    while k < fin:
        if callado[k]:
            j = k
            while j < fin and callado[j]:
                j += 1
            if runs and k - runs[-1][1] <= round(UNIR_S / HOP_S):
                runs[-1][1] = j
            else:
                runs.append([k, j])
            k = j
        else:
            k += 1
    largas = [(a * hop, b * hop) for a, b in runs if (b - a) * HOP_S >= PAUSA_MIN_S]
    return largas, inicio * hop, fin * hop


def lugares(runs: list[tuple[int, int]], inicio: int, fin: int) -> list[float]:
    """Each pause's share of the speech heard before it (pauses do not count as speech)."""
    habla = (fin - inicio) - sum(b - a for a, b in runs)
    out, callado = [], 0
    for a, b in runs:
        out.append((a - inicio - callado) / habla)
        callado += b - a
    return out


def alinear(ms: list[Marca], lugar: list[float], largo_s: list[float]) -> list[int]:
    """For each `fin` mark, in order, the index of its pause.

    Raises TomaRechazada when there are fewer pauses than sentence ends.
    """
    M, K = len(ms), len(lugar)
    inf = float("inf")
    dp = np.full((M + 1, K + 1), inf)
    paso = np.zeros((M + 1, K + 1), dtype=np.int8)  # 1 pause alone, 2 comma alone, 3 pair
    dp[0, 0] = 0.0
    for i in range(M + 1):
        for k in range(K + 1):
            c = dp[i, k]
            if c == inf:
                continue
            if k < K:  # pause k, unpaired
                v = c + (largo_s[k] / PAUSA_LIBRE_S) ** 2
                if v < dp[i, k + 1]:
                    dp[i, k + 1], paso[i, k + 1] = v, 1
            if i < M and not ms[i].fin:  # comma i, said without a pause
                v = c + SALTO_COMA
                if v < dp[i + 1, k]:
                    dp[i + 1, k], paso[i + 1, k] = v, 2
            if i < M and k < K:  # mark i in pause k
                v = c + ((lugar[k] - ms[i].lugar) / SIGMA) ** 2
                if ms[i].fin and largo_s[k] < FIN_MIN_S:
                    v += ((FIN_MIN_S - largo_s[k]) / 0.05) ** 2
                if v < dp[i + 1, k + 1]:
                    dp[i + 1, k + 1], paso[i + 1, k + 1] = v, 3
    if dp[M, K] == inf:
        raise TomaRechazada(f"{sum(m.fin for m in ms)} cortes de frase y {K} pausas")
    par: dict[int, int] = {}
    i, k = M, K
    while i or k:
        p = paso[i, k]
        if p == 1:
            k -= 1
        elif p == 2:
            i -= 1
        else:
            i, k = i - 1, k - 1
            par[i] = k
    return [par[i] for i in range(M) if ms[i].fin]


def cortes(x: np.ndarray, sr: int, frases: list[str]) -> list[tuple[int, int]]:
    """The pause after each sentence but the last, as (start, end) samples, checked.

    Raises TomaRechazada when a cut lands far from its sentence end or a
    sentence's rate is far from the take's.
    """
    runs, inicio, fin = pausas(x, sr)
    lugar = lugares(runs, inicio, fin)
    ms = marcas(frases)
    elegidas = alinear(ms, lugar, [(b - a) / sr for a, b in runs])
    fines = [m for m in ms if m.fin]
    total = sum(letras(f) for f in frases)
    for i, (m, k) in enumerate(zip(fines, elegidas)):
        # a very short sentence next to the cut («Sí.») moves it most: its letters
        # say little of its time; the widening is capped, or a cut at a comma of
        # two long sentences would pass
        tol = ERROR_MAX
        corta = min(letras(frases[i]), letras(frases[i + 1]))
        if corta < RITMO_LETRAS:
            tol = max(ERROR_MAX, min(corta / total, ENSANCHE_MAX))
        if abs(lugar[k] - m.lugar) > tol:
            raise TomaRechazada(
                f"un corte cae a {lugar[k]:.3f} del habla y la frase termina a {m.lugar:.3f}"
            )
    bordes = [inicio] + [x for k in elegidas for x in runs[k]] + [fin]
    tramos = list(zip(bordes[::2], bordes[1::2]))
    ritmo = sum(letras(f) for f in frases) / sum(b - a for a, b in tramos)
    for f, (a, b) in zip(frases, tramos):
        if letras(f) < RITMO_LETRAS:
            continue
        r = letras(f) / (b - a) / ritmo
        if not RITMO[0] <= r <= RITMO[1]:
            raise TomaRechazada(f"«{f}» va a {r:.2f}× el ritmo de la toma")
    return [runs[k] for k in elegidas]


def a_ceros(x: np.ndarray, sr: int, frases: list[str], silencio: float) -> np.ndarray:
    """The take as int16, trimmed to its speech, with each sentence cut an exact zero run.

    Postconditions (guion's `Voice` contract, R-0019): between every two
    sentences a run of zeros at least `silencio` s long (the voice's own pause
    when longer); no other zero sample; the take starts and ends at its speech.
    Raises TomaRechazada as `cortes` does.
    """
    runs = cortes(x, sr, frases)
    _, inicio, fin = pausas(x, sr)
    f = max(1, int(FUNDIDO_S * sr))
    v = x.astype(np.float64) * 32767
    # speech pieces: each fades out over the first FUNDIDO_S of its pause and
    # in over the last, so the zeros never cut a sound
    bordes = [max(0, inicio - f)]
    for a, b in runs:
        bordes += [a + f, b - f]
    bordes.append(min(len(v), fin + f))
    piezas = []
    for n, (a, b) in enumerate(zip(bordes[::2], bordes[1::2])):
        voz = v[a:b].copy()
        g = min(f, len(voz))
        voz[:g] *= np.linspace(0.0, 1.0, g)
        voz[len(voz) - g :] *= np.linspace(1.0, 0.0, g)
        piezas.append(_sin_ceros(voz))
        if n < len(runs):
            a0, b0 = runs[n]
            piezas.append(np.zeros(max(b0 - a0 - 2 * f, int(np.ceil(silencio * sr))), np.int16))
    return np.concatenate(piezas)


def _sin_ceros(v: np.ndarray) -> np.ndarray:
    """int16, with every zero raised to 1 (-90 dBFS, inaudible): only cuts are zeros."""
    out = np.clip(np.round(v), -32768, 32767).astype(np.int16)
    out[out == 0] = 1
    return out
