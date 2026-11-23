//! Highlights, Shadows, Whites and Blacks past ±100 (process V11's extended range, −200..200): an amount past
//! ±100 stacks on the full ±100 effect and keeps tone order; earlier processes render at most ±100.

use lightcraft_develop::{DevelopSettings, Process, controls};
use lightcraft_raster::{Rgb32f, Rgba8};

use crate::{RenderRequest, SourceInfo, render};

const W: usize = 256;

/// A horizontal ramp from 9 stops below grey to 3 above, through a camera tone curve, so the calibrated Blacks
/// operators and the curve stage both apply.
fn shot(process: Process, id: &str, v: f64) -> Rgba8 {
    let curve = crate::tone::CameraTone::new(std::array::from_fn(|i| {
        let x = 0.004 * 1.18f32.powi(i as i32);
        [x, 1.0 - (-2.0 * x).exp()]
    }))
    .expect("curve");
    let info = SourceInfo { raw: true, camera_tone: Some(curve), ..Default::default() };
    let src = Rgb32f::from_fn(W, 8, |x, _| [0.18 * (-9.0 + 12.0 * x as f32 / (W - 1) as f32).exp2(); 3]);
    let mut s = DevelopSettings { process: process.version(), ..Default::default() };
    controls::set(&mut s, id, v);
    render(&src, &info, &s, &RenderRequest::fit(W, 8)).image
}

/// The green channel along the ramp's middle row.
fn row(img: &Rgba8) -> Vec<f32> {
    (0..W).map(|x| f32::from(img.get(x, 4)[1])).collect()
}

fn mean(r: &[f32]) -> f32 {
    r.iter().sum::<f32>() / r.len() as f32
}

const SLIDERS: [&str; 4] = ["light.highlights", "light.shadows", "light.whites", "light.blacks"];

#[test]
fn past_100_the_sliders_go_further_the_same_way_and_keep_tone_order() {
    for id in SLIDERS {
        let base = mean(&row(&shot(Process::V11, id, 0.0)));
        for sign in [-1.0, 1.0] {
            let at = |v: f64| mean(&row(&shot(Process::V11, id, sign * v)));
            let (m100, m150, m200) = (at(100.0), at(150.0), at(200.0));
            let (d100, d150, d200) = ((m100 - base) * sign as f32, (m150 - base) * sign as f32, (m200 - base) * sign as f32);
            assert!(d100 > 0.0 && d150 > d100 && d200 > d150, "{id} {sign}: moves {d100}, {d150}, {d200}");
            // the ramp stays a ramp: no tone reversal at the end of the range (8-bit, one code of slack)
            let r = row(&shot(Process::V11, id, sign * 200.0));
            assert!(r.windows(2).all(|p| p[1] >= p[0] - 1.0), "{id} {}: tone order reversed", sign * 200.0);
        }
    }
}

#[test]
fn processes_before_v11_render_at_most_100() {
    for id in SLIDERS {
        for v in [-160.0, 160.0] {
            let at_100 = shot(Process::V10, id, 100.0f64.copysign(v));
            assert!(shot(Process::V10, id, v) == at_100, "V10 {id} {v} renders as ±100");
        }
    }
}

#[test]
fn blacks_past_100_is_the_full_operator_then_the_rest() {
    use crate::light_v3::{MAX_OPS, blacks_amounts};
    assert_eq!(blacks_amounts(-60.0, Process::V11), vec![-60.0]);
    assert_eq!(blacks_amounts(-150.0, Process::V11), vec![-100.0, -50.0]);
    assert_eq!(blacks_amounts(250.0, Process::V11), vec![100.0, 100.0], "clamped to ±200");
    assert_eq!(blacks_amounts(-150.0, Process::V10), vec![-100.0], "±100 before V11");
    // Contrast and two Blacks operators: what the GPU kernel takes
    assert!(1 + blacks_amounts(-200.0, Process::V11).len() <= MAX_OPS);
}
