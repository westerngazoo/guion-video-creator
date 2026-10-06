//! R-0005 — la entrega: una prueba por criterio de aceptación.
//!
//! Necesitan ffmpeg con libx264, `loudnorm`, `ebur128` y `alimiter`: el de
//! PATH o el de `GUION_FFMPEG`. Sin ffmpeg fallan, y el error dice cómo
//! dárselo: un criterio de entrega que no se puede comprobar no está
//! cumplido.

use std::path::{Path, PathBuf};
use std::process::Command;

use guion_encode::{
    encode_ordered, encode_ordered_with, encode_ppm_dir, measure, mux_at_loudness, EncodeError,
    Encoder, LoudnessTarget, VideoSpec,
};

const W: u32 = 64;
const H: u32 = 48;
const SPEC: VideoSpec = VideoSpec {
    width: W,
    height: H,
    fps: 30.0,
};

fn ff() -> String {
    guion_encode::ffmpeg::ffmpeg()
        .expect("estas pruebas necesitan ffmpeg (en PATH o en GUION_FFMPEG)")
}

/// Un directorio nuevo y vacío para cada prueba.
fn dir(nombre: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-entrega-{}-{nombre}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Un cuadro que cambia con su índice: un degradado que se corre.
fn cuadro(i: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(SPEC.frame_len());
    for y in 0..H as usize {
        for x in 0..W as usize {
            v.push(((x * 4 + i * 7) % 256) as u8);
            v.push(((y * 5 + i * 3) % 256) as u8);
            v.push(((x + y + i) % 256) as u8);
        }
    }
    v
}

fn liso(rgb: [u8; 3]) -> Vec<u8> {
    rgb.iter().copied().cycle().take(SPEC.frame_len()).collect()
}

fn escribir(out: &Path, cuadros: &[Vec<u8>]) {
    let mut enc = Encoder::start(out, SPEC).unwrap();
    for c in cuadros {
        enc.push(c).unwrap();
    }
    assert_eq!(enc.finish().unwrap(), cuadros.len());
}

/// Los cuadros de un MP4, decodificados como BT.709 en rango de TV a RGB.
fn decodificar(mp4: &Path) -> Vec<Vec<u8>> {
    let out = Command::new(ff())
        .args(["-v", "error", "-i"])
        .arg(mp4)
        .args([
            "-vf",
            "scale=in_color_matrix=bt709:in_range=tv,format=rgb24",
            "-f",
            "rawvideo",
            "-",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
        .chunks(SPEC.frame_len())
        .map(<[u8]>::to_vec)
        .collect()
}

/// Las cajas de primer nivel de un MP4, en orden.
fn cajas(mp4: &Path) -> Vec<String> {
    let b = std::fs::read(mp4).unwrap();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 8 <= b.len() {
        let mut largo = u32::from_be_bytes(b[i..i + 4].try_into().unwrap()) as usize;
        if largo == 1 {
            largo = u64::from_be_bytes(b[i + 8..i + 16].try_into().unwrap()) as usize;
        }
        out.push(String::from_utf8_lossy(&b[i + 4..i + 8]).into_owned());
        if largo < 8 {
            break;
        }
        i += largo;
    }
    out
}

fn indice(cajas: &[String], nombre: &str) -> usize {
    cajas
        .iter()
        .position(|c| c == nombre)
        .unwrap_or_else(|| panic!("sin caja {nombre}: {cajas:?}"))
}

/// AC1: el MP4 dice que es BT.709 en rango de TV.
#[test]
fn ac1_el_video_va_etiquetado_bt709() {
    let d = dir("ac1");
    let mp4 = d.join("v.mp4");
    escribir(&mp4, &(0..8).map(cuadro).collect::<Vec<_>>());
    let info = Command::new(ff())
        .arg("-hide_banner")
        .arg("-i")
        .arg(&mp4)
        .output()
        .unwrap();
    let info = String::from_utf8_lossy(&info.stderr);
    assert!(info.contains("yuv420p(tv, bt709, progressive)"), "{info}");
}

/// AC2: un color llega como se mandó (±3 por canal) al decodificarlo como
/// BT.709. Con la matriz BT.601 de antes, el oro salía a 5 o más.
#[test]
fn ac2_el_color_llega_como_se_mando() {
    let d = dir("ac2");
    for (k, rgb) in [
        [227, 169, 58],
        [12, 12, 14],
        [240, 236, 226],
        [200, 40, 40],
        [58, 110, 170],
    ]
    .into_iter()
    .enumerate()
    {
        let mp4 = d.join(format!("c{k}.mp4"));
        escribir(&mp4, &vec![liso(rgb); 4]);
        let f = &decodificar(&mp4)[0];
        let centro = ((H as usize / 2) * W as usize + W as usize / 2) * 3;
        let got = [f[centro], f[centro + 1], f[centro + 2]];
        for c in 0..3 {
            assert!(
                (got[c] as i32 - rgb[c] as i32).abs() <= 3,
                "{rgb:?} llegó como {got:?}"
            );
        }
    }
}

/// AC3: el índice (`moov`) va antes de los datos (`mdat`), en memoria y desde
/// PPM sin audio, que antes salía sin `+faststart`.
#[test]
fn ac3_el_indice_va_antes_que_los_datos() {
    let d = dir("ac3");
    let mp4 = d.join("v.mp4");
    escribir(&mp4, &(0..8).map(cuadro).collect::<Vec<_>>());
    let c = cajas(&mp4);
    assert!(indice(&c, "moov") < indice(&c, "mdat"), "{c:?}");

    let ppm = d.join("frames");
    std::fs::create_dir_all(&ppm).unwrap();
    for i in 0..8 {
        let mut b = format!("P6\n{W} {H}\n255\n").into_bytes();
        b.extend(cuadro(i));
        std::fs::write(ppm.join(format!("frame_{i:05}.ppm")), b).unwrap();
    }
    let desde_ppm = d.join("p.mp4");
    encode_ppm_dir(&ppm, 30.0, &desde_ppm, None).unwrap();
    let c = cajas(&desde_ppm);
    assert!(indice(&c, "moov") < indice(&c, "mdat"), "{c:?}");
}

/// AC4: los cuadros en memoria dan el mismo video que los mismos cuadros
/// como PPM en disco.
#[test]
fn ac4_en_memoria_igual_que_en_disco() {
    let d = dir("ac4");
    let cuadros: Vec<Vec<u8>> = (0..12).map(cuadro).collect();
    let mem = d.join("m.mp4");
    escribir(&mem, &cuadros);
    let ppm = d.join("frames");
    std::fs::create_dir_all(&ppm).unwrap();
    for (i, c) in cuadros.iter().enumerate() {
        let mut b = format!("P6\n{W} {H}\n255\n").into_bytes();
        b.extend(c);
        std::fs::write(ppm.join(format!("frame_{i:05}.ppm")), b).unwrap();
    }
    let disco = d.join("d.mp4");
    encode_ppm_dir(&ppm, 30.0, &disco, None).unwrap();
    let (a, b) = (decodificar(&mem), decodificar(&disco));
    assert_eq!(a.len(), 12);
    assert!(a == b, "los cuadros decodificados difieren");
}

/// AC5: uno o cuatro hilos, los mismos bytes; y están todos los cuadros.
#[test]
fn ac5_uno_o_cuatro_hilos_dan_los_mismos_bytes() {
    let d = dir("ac5");
    let n = 37; // no es múltiplo de 4: el último hilo trabaja menos
    let render = |i: usize| -> Result<Vec<u8>, String> { Ok(cuadro(i)) };
    let (uno, cuatro) = (d.join("1.mp4"), d.join("4.mp4"));
    assert_eq!(encode_ordered(&uno, SPEC, n, 1, render).unwrap(), n);
    assert_eq!(encode_ordered(&cuatro, SPEC, n, 4, render).unwrap(), n);
    assert!(
        std::fs::read(&uno).unwrap() == std::fs::read(&cuatro).unwrap(),
        "1 y 4 hilos difieren"
    );
    assert_eq!(decodificar(&cuatro).len(), n);
}

/// Un WAV de 16 bits, mono, 48 kHz: un tono suave con golpes secos cada
/// medio segundo. Mucho factor de cresta, como el audio de los reels.
fn wav_con_golpes(path: &Path, segundos: f64) {
    let sr = 48_000u32;
    let n = (segundos * sr as f64) as usize;
    let mut datos = Vec::with_capacity(n * 2);
    for k in 0..n {
        let t = k as f64 / sr as f64;
        let tono = 0.03 * (std::f64::consts::TAU * 220.0 * t).sin();
        let desde = t % 0.5;
        let golpe = 0.95 * (-desde / 0.012).exp() * (std::f64::consts::TAU * 90.0 * desde).sin();
        let s = ((tono + golpe).clamp(-1.0, 1.0) * 32767.0) as i16;
        datos.extend_from_slice(&s.to_le_bytes());
    }
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + datos.len() as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&sr.to_le_bytes());
    b.extend_from_slice(&(sr * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(datos.len() as u32).to_le_bytes());
    b.extend(datos);
    std::fs::write(path, b).unwrap();
}

/// AC6: el audio queda a −14 LUFS ± 0.5 con pico real ≤ −1 dBTP, medido en
/// la salida; y la misma entrada da los mismos bytes.
#[test]
fn ac6_el_audio_queda_a_menos_14_lufs() {
    let d = dir("ac6");
    let video = d.join("v.mp4");
    escribir(&video, &(0..120).map(cuadro).collect::<Vec<_>>());
    let wav = d.join("a.wav");
    wav_con_golpes(&wav, 4.0);
    let antes = measure(&wav).unwrap();
    assert!(
        antes.integrated_lufs < -16.0,
        "la prueba empieza lejos de la meta: {antes:?}"
    );

    let out = d.join("final.mp4");
    let m = mux_at_loudness(&video, &wav, &out, LoudnessTarget::REELS).unwrap();
    assert!((m.integrated_lufs + 14.0).abs() <= 0.5, "{m:?}");
    assert!(m.true_peak_dbtp <= -1.0, "{m:?}");
    assert_eq!(
        measure(&out).unwrap(),
        m,
        "lo que devuelve es lo que mide la salida"
    );

    let otra = d.join("otra.mp4");
    mux_at_loudness(&video, &wav, &otra, LoudnessTarget::REELS).unwrap();
    assert!(
        std::fs::read(&out).unwrap() == std::fs::read(&otra).unwrap(),
        "no es determinista"
    );
}

/// AC7: cada error dice qué y dónde, y ninguno deja archivo.
#[test]
fn ac7_errores_con_nombre_y_sin_archivo() {
    let d = dir("ac7");
    let restos = |d: &Path| std::fs::read_dir(d).unwrap().count();

    // lados impares: yuv420p no puede
    let impar = VideoSpec { width: 63, ..SPEC };
    let e = Encoder::start(&d.join("x.mp4"), impar)
        .err()
        .expect("lados impares");
    assert!(
        matches!(&e, EncodeError::InvalidInput(m) if m.contains("63×48")),
        "{e}"
    );

    // fps que no son
    let e = Encoder::start(&d.join("x.mp4"), VideoSpec { fps: 0.0, ..SPEC })
        .err()
        .expect("fps 0");
    assert!(matches!(e, EncodeError::InvalidInput(_)), "{e}");

    // un cuadro del tamaño equivocado: dice cuál, cuánto trajo y cuánto lleva
    let mut enc = Encoder::start(&d.join("x.mp4"), SPEC).unwrap();
    enc.push(&cuadro(0)).unwrap();
    let e = enc.push(&[0u8; 10]).unwrap_err();
    assert!(
        matches!(&e, EncodeError::InvalidInput(m) if m.contains("cuadro 1") && m.contains("10 bytes") && m.contains("9216")),
        "{e}"
    );
    drop(enc);
    assert_eq!(restos(&d), 0, "un Encoder soltado no deja nada");

    // el render falla en el cuadro 3: el error lo dice y no queda video
    let out = d.join("r.mp4");
    let e = encode_ordered(&out, SPEC, 10, 3, |i| {
        if i == 3 {
            Err(format!("sin escena para t = {}", i as f64 / 30.0))
        } else {
            Ok(cuadro(i))
        }
    })
    .unwrap_err();
    assert!(
        matches!(&e, EncodeError::Render { frame: 3, message } if message.contains("sin escena")),
        "{e}"
    );
    assert!(!out.exists());
    assert_eq!(restos(&d), 0, "sin salida a medias");
}

/// AC8: cada hilo arma su estado una vez y lo usa en todos sus cuadros; el
/// video es el mismo que sin estado; y si armarlo falla, el error dice el
/// hilo, sale como su primer cuadro, y no queda archivo.
#[test]
fn ac8_cada_hilo_con_su_lienzo() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let d = dir("ac8");
    let (n, hilos) = (23, 4);
    let armados = AtomicUsize::new(0);
    let con = d.join("con.mp4");
    let escritos = encode_ordered_with(
        &con,
        SPEC,
        n,
        hilos,
        |h| {
            armados.fetch_add(1, Ordering::SeqCst);
            Ok::<_, String>((h, 0usize, Vec::<u8>::with_capacity(SPEC.frame_len())))
        },
        |(h, hechos, lienzo), i| {
            assert_eq!(i % hilos, *h, "el cuadro {i} llegó al hilo {h}");
            *hechos += 1;
            lienzo.clear();
            lienzo.extend(cuadro(i));
            Ok(lienzo.clone())
        },
    )
    .unwrap();
    assert_eq!(escritos, n);
    assert_eq!(armados.load(Ordering::SeqCst), hilos, "un estado por hilo");
    let sin = d.join("sin.mp4");
    encode_ordered(&sin, SPEC, n, hilos, |i| Ok::<_, String>(cuadro(i))).unwrap();
    assert!(
        std::fs::read(&con).unwrap() == std::fs::read(&sin).unwrap(),
        "el estado no cambia el video"
    );

    let falla = d.join("falla.mp4");
    let e = encode_ordered_with(
        &falla,
        SPEC,
        n,
        hilos,
        |h| {
            if h == 2 {
                Err("sin carpeta".to_string())
            } else {
                Ok(())
            }
        },
        |_, i| Ok(cuadro(i)),
    )
    .unwrap_err();
    assert!(
        matches!(&e, EncodeError::Render { frame: 2, message } if message.contains("hilo 2") && message.contains("sin carpeta")),
        "{e}"
    );
    assert!(!falla.exists());
}
