//! What the gate found: one [`Finding`] per check, as a short JSON and a
//! human summary of 40 lines or fewer (R-0012 AC8).

use serde::Serialize;

/// The result of one check.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
    /// The check's name (`subtitles`, `red`, `safe-area`…).
    pub check: String,
    pub pass: bool,
    /// One line: what was measured, or why it failed.
    pub detail: String,
    /// Where: times (s) and frames of what failed, the worst first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub at: Vec<String>,
}

impl Finding {
    pub fn pass(check: &str, detail: impl Into<String>) -> Finding {
        Finding {
            check: check.into(),
            pass: true,
            detail: detail.into(),
            at: Vec::new(),
        }
    }

    pub fn fail(check: &str, detail: impl Into<String>, at: Vec<String>) -> Finding {
        Finding {
            check: check.into(),
            pass: false,
            detail: detail.into(),
            at,
        }
    }

    /// A check that passes when `at` is empty.
    pub fn from_misses(
        check: &str,
        ok: impl Into<String>,
        bad: impl Into<String>,
        at: Vec<String>,
    ) -> Finding {
        if at.is_empty() {
            Finding::pass(check, ok)
        } else {
            Finding::fail(check, bad, at)
        }
    }
}

/// The gate's report on one reel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub post: String,
    pub pass: bool,
    /// Wall-clock time the gate took, s.
    pub seconds: f64,
    pub findings: Vec<Finding>,
}

/// How many locations a finding keeps (the JSON stays short).
pub const MAX_AT: usize = 12;

impl Report {
    pub fn new(post: String, findings: Vec<Finding>, seconds: f64) -> Report {
        let pass = findings.iter().all(|f| f.pass);
        Report {
            post,
            pass,
            seconds,
            findings,
        }
    }

    /// The JSON, pretty-printed (keys in a fixed order: the struct's).
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a report serializes");
        s.push('\n');
        s
    }

    /// The human summary: a header, one line per check, at most three places
    /// per failure, and the verdict. Never more than 40 lines.
    pub fn summary(&self) -> String {
        let mut lines = vec![format!("guion reel check {}", self.post)];
        for f in &self.findings {
            let mark = if f.pass { "PASS" } else { "FAIL" };
            lines.push(format!("{mark} {:<14} {}", f.check, f.detail));
            if !f.pass {
                let shown: Vec<&str> = f.at.iter().take(3).map(String::as_str).collect();
                if !shown.is_empty() {
                    let more = f.at.len().saturating_sub(3);
                    let tail = if more > 0 {
                        format!(" (+{more})")
                    } else {
                        String::new()
                    };
                    lines.push(format!("     {:<14} {}{tail}", "", shown.join("; ")));
                }
            }
        }
        let failed = self.findings.iter().filter(|f| !f.pass).count();
        lines.truncate(39);
        lines.push(format!(
            "{}: {} checks, {failed} FAIL, {:.1} s",
            if self.pass { "PASS" } else { "FAIL" },
            self.findings.len(),
            self.seconds
        ));
        lines.join("\n") + "\n"
    }
}

/// `12.30 s (f369)`: a time and its frame.
pub fn at_frame(frame: usize, fps: f64) -> String {
    format!("{:.2} s (f{frame})", frame as f64 / fps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_never_passes_forty_lines() {
        let findings = (0..60)
            .map(|i| Finding::fail("x", format!("{i}"), vec!["a".into(); 5]))
            .collect();
        let r = Report::new("p".into(), findings, 1.0);
        assert!(r.summary().lines().count() <= 40);
        assert!(!r.pass);
        assert!(r.summary().lines().last().unwrap().starts_with("FAIL"));
    }
}
