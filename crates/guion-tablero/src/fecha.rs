//! Today's date, `YYYY-MM-DD`, without a date crate.

use std::time::{SystemTime, UNIX_EPOCH};

/// Today, in the local zone when `date` is there (the owner's day, not
/// UTC's), else in UTC.
pub fn hoy() -> String {
    if let Ok(out) = std::process::Command::new("date").arg("+%F").output() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && es_fecha(&s) {
            return s;
        }
    }
    let dias = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let (y, m, d) = civil(dias);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 → (year, month, day) (H. Hinnant's algorithm).
pub fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// `YYYY-MM-DD`, with a month and a day in range.
pub fn es_fecha(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| s.get(r).and_then(|x| x.parse::<u32>().ok());
    matches!((num(0..4), num(5..7), num(8..10)), (Some(_), Some(m), Some(d)) if (1..=12).contains(&m) && (1..=31).contains(&d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_to_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_744), (2026, 10, 18));
        assert!(es_fecha("2026-10-18"));
        assert!(!es_fecha("2026-13-01"));
        assert!(!es_fecha("18/10/2026"));
    }
}
