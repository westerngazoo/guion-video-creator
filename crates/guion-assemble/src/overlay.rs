use guion_brand::Theme;
use guion_core::Screenplay;
use motoreel::{Align, Anchor, Label, Pt2, ScreenAnchor};

/// Hook + footer screen labels for the current time `t`.
pub fn labels(sp: &Screenplay, theme: &Theme, t: f64) -> Vec<Label> {
    let mut out = Vec::new();
    if let Some(hook) = &sp.hook {
        if t >= hook.at.start && t <= hook.at.end {
            let mut lbl = Label::new(hook.text.clone(), Anchor::Screen(ScreenAnchor::TopCentre));
            lbl = lbl
                .with_align(Align::Center)
                .with_size(0.12)
                .with_offset(Pt2 { x: 0.0, y: -0.35 });
            lbl.style.stroke = theme.stroke_color("accent2", None);
            out.push(lbl);
        }
    }
    for cue in &sp.narration {
        if t >= cue.at.start && t <= cue.at.end {
            let mut lbl = Label::new(cue.text.clone(), Anchor::Screen(ScreenAnchor::Centre));
            lbl = lbl
                .with_align(Align::Center)
                .with_size(0.055)
                .with_offset(Pt2 { x: 0.0, y: 0.05 });
            lbl.style.stroke = theme.stroke_color("paper", None);
            out.push(lbl);
        }
    }
    if let Some(footer) = &sp.footer {
        let mut concept = Label::new(
            footer.concept.clone(),
            Anchor::Screen(ScreenAnchor::BottomCentre),
        );
        concept = concept
            .with_align(Align::Center)
            .with_size(0.05)
            .with_offset(Pt2 { x: 0.0, y: 0.45 });
        concept.style.stroke = theme.stroke_color("ink", None);
        out.push(concept);

        let mut detail = Label::new(
            footer.detail.clone(),
            Anchor::Screen(ScreenAnchor::BottomCentre),
        );
        detail = detail
            .with_align(Align::Center)
            .with_size(0.04)
            .with_offset(Pt2 { x: 0.0, y: 0.38 });
        detail.style.stroke = theme.stroke_color("ink", None);
        out.push(detail);
    }
    out
}
