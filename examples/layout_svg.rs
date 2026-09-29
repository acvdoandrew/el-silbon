//! Dev tool: dump the authored layout as a top-down SVG for review.
//!
//! ```sh
//! cargo run --release --locked --example layout_svg -- target/layout.svg
//! ```
//!
//! Every shape is read from `Layout` (the same data collision, sight, routes
//! and visuals use), so what this draws is exactly what the game resolves.

use std::fmt::Write as _;

use el_silbon::geometry::{Blocker, BlockerKind, Layout, Shape, Sight};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "target/layout.svg".into());
    let l = Layout::new();
    let scale = 8.0_f32;
    let b = l.bounds;
    let pad = 14.0_f32;
    let (x0, z0) = (b.min.x - pad, b.min.y - pad);
    let (w, h) = (b.max.x - b.min.x + 2.0 * pad, b.max.y - b.min.y + 2.0 * pad);
    let px = |x: f32| (x - x0) * scale;
    let pz = |z: f32| (z - z0) * scale;
    let mut s = String::new();
    let _ = writeln!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" font-family="sans-serif">"##,
        w * scale,
        h * scale,
        w * scale,
        h * scale
    );
    let _ = writeln!(s, r##"<rect width="100%" height="100%" fill="#1b2418"/>"##);
    // Grid every 10 m.
    let mut gx = (x0 / 10.0).floor() * 10.0;
    while gx < x0 + w {
        let _ = writeln!(
            s,
            r##"<line x1="{0}" y1="0" x2="{0}" y2="{1}" stroke="#2b3a2a" stroke-width="1"/><text x="{0}" y="10" fill="#6b7f68" font-size="9">{2}</text>"##,
            px(gx),
            h * scale,
            gx as i32
        );
        gx += 10.0;
    }
    let mut gz = (z0 / 10.0).floor() * 10.0;
    while gz < z0 + h {
        let _ = writeln!(
            s,
            r##"<line x1="0" y1="{0}" x2="{1}" y2="{0}" stroke="#2b3a2a" stroke-width="1"/><text x="2" y="{0}" fill="#6b7f68" font-size="9">{2}</text>"##,
            pz(gz),
            w * scale,
            gz as i32
        );
        gz += 10.0;
    }
    let _ = writeln!(
        s,
        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="#ffffff" stroke-dasharray="6 4"/>"##,
        px(b.min.x),
        pz(b.min.y),
        (b.max.x - b.min.x) * scale,
        (b.max.y - b.min.y) * scale
    );
    // Road.
    let _ = writeln!(
        s,
        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#5a4b39"/>"##,
        px(l.road.min.x.max(b.min.x)),
        pz(l.road.min.y),
        (l.road.max.x.min(b.max.x) - l.road.min.x.max(b.min.x)) * scale,
        (l.road.max.y - l.road.min.y) * scale
    );
    {
        let d = &l.district;
        for r in &d.grass {
            let _ = writeln!(
                s,
                r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#3c5a25" fill-opacity="0.55"/>"##,
                px(r.min.x),
                pz(r.min.y),
                (r.max.x - r.min.x) * scale,
                (r.max.y - r.min.y) * scale
            );
        }
        for r in &d.water {
            let _ = writeln!(
                s,
                r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#0f3550" fill-opacity="0.85"/>"##,
                px(r.min.x),
                pz(r.min.y),
                (r.max.x - r.min.x) * scale,
                (r.max.y - r.min.y) * scale
            );
        }
        for r in &d.shallows {
            let _ = writeln!(
                s,
                r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#3d8aa0" fill-opacity="0.6"/>"##,
                px(r.min.x),
                pz(r.min.y),
                (r.max.x - r.min.x) * scale,
                (r.max.y - r.min.y) * scale
            );
        }
        for sf in &d.surfaces {
            let r = sf.rect;
            let _ = writeln!(
                s,
                r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#a07a44" fill-opacity="0.8"/>"##,
                px(r.min.x),
                pz(r.min.y),
                (r.max.x - r.min.x) * scale,
                (r.max.y - r.min.y) * scale
            );
        }
        for route in &d.routes {
            let pts: Vec<String> = route
                .points
                .iter()
                .map(|p| format!("{:.1},{:.1}", px(p.x), pz(p.y)))
                .collect();
            let _ = writeln!(
                s,
                r##"<polyline points="{}" fill="none" stroke="#d8c48a" stroke-opacity="0.45" stroke-width="{}"/>"##,
                pts.join(" "),
                route.width * scale
            );
        }
    }
    for bl in &l.blockers {
        draw_blocker(&mut s, bl, &px, &pz, scale);
    }
    for (i, a) in l.patrol.nodes.iter().enumerate() {
        let _ = writeln!(
            s,
            r##"<circle cx="{}" cy="{}" r="5" fill="#ff5050"/><text x="{}" y="{}" fill="#ff9090" font-size="10">A{i}</text>"##,
            px(a.x),
            pz(a.y),
            px(a.x) + 6.0,
            pz(a.y) - 4.0
        );
    }
    for &(i, j) in &l.patrol.edges {
        let (a, c) = (l.patrol.nodes[i], l.patrol.nodes[j]);
        let _ = writeln!(
            s,
            r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#ff5050" stroke-opacity="0.5" stroke-width="1.5" stroke-dasharray="4 3"/>"##,
            px(a.x),
            pz(a.y),
            px(c.x),
            pz(c.y)
        );
    }
    {
        let d = &l.district;
        for m in &d.landmarks {
            let _ = writeln!(
                s,
                r##"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="#ffd070" stroke-opacity="0.5"/><text x="{}" y="{}" fill="#ffd070" font-size="13" font-weight="bold">{}</text>"##,
                px(m.center.x),
                pz(m.center.y),
                m.clearing * scale,
                px(m.center.x) - 24.0,
                pz(m.center.y) - 4.0,
                m.name
            );
        }
        let mark = |s: &mut String, x: f32, z: f32, color: &str, label: &str| {
            let _ = writeln!(
                s,
                r##"<circle cx="{}" cy="{}" r="4" fill="{color}"/><text x="{}" y="{}" fill="{color}" font-size="9">{label}</text>"##,
                px(x),
                pz(z),
                px(x) + 5.0,
                pz(z) + 3.0
            );
        };
        for (i, r) in d.relics.iter().enumerate() {
            mark(&mut s, r.x, r.z, "#ffffff", &format!("bone{i}"));
        }
        for (i, r) in d.aji.iter().enumerate() {
            mark(&mut s, r.x, r.z, "#ff7a3a", &format!("aji{i}"));
        }
        for n in &d.notes {
            mark(&mut s, n.pos.x, n.pos.z, "#c0c0ff", "note");
        }
        for lamp in &d.lamps {
            mark(
                &mut s,
                lamp.pos.x,
                lamp.pos.z,
                if lamp.powered { "#fff090" } else { "#ffa040" },
                "",
            );
        }
        mark(&mut s, d.pump.x, d.pump.z, "#40ff90", "pump");
        mark(&mut s, d.ignition.x, d.ignition.z, "#40ff90", "ignition");
        mark(&mut s, d.beacon.x, d.beacon.z, "#40ff90", "beacon");
        mark(&mut s, l.ceiba.offering.x, l.ceiba.offering.z, "#40ff90", "altar");
        mark(&mut s, l.spawn.x, l.spawn.y, "#40a0ff", "spawn");
        for c in d.cows() {
            mark(&mut s, c.center.x, c.center.y, "#e0d0c0", "");
        }
    }
    let _ = writeln!(s, "</svg>");
    std::fs::write(&out, s).expect("write svg");
    println!("wrote {out} ({} blockers)", l.blockers.len());
}

fn draw_blocker(s: &mut String, bl: &Blocker, px: &dyn Fn(f32) -> f32, pz: &dyn Fn(f32) -> f32, scale: f32) {
    let (fill, opacity) = match (bl.kind, bl.sight) {
        (BlockerKind::Wall, _) => ("#c9b28a", 0.95),
        (BlockerKind::Bank, _) => ("#000000", 0.0),
        (BlockerKind::Fence | BlockerKind::Post, _) => ("#8a6a3c", 0.9),
        (BlockerKind::Trunk, Sight::Blocks) => ("#2f5a2a", 0.9),
        (_, Sight::Blocks) => ("#a0a0a0", 0.9),
        _ => ("#7d7d70", 0.8),
    };
    match bl.shape {
        Shape::Rect(r) => {
            let _ = writeln!(
                s,
                r##"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}" fill-opacity="{opacity}"/>"##,
                px(r.min.x),
                pz(r.min.y),
                (r.max.x - r.min.x) * scale,
                (r.max.y - r.min.y) * scale
            );
        }
        Shape::Circle { center, radius } => {
            let _ = writeln!(
                s,
                r##"<circle cx="{}" cy="{}" r="{}" fill="{fill}" fill-opacity="{opacity}"/>"##,
                px(center.x),
                pz(center.y),
                (radius * scale).max(1.5)
            );
        }
    }
}
