//! A default UI scale for X11 desktops that don't set one.
//!
//! On X11, windows are scaled by `Xft.dpi / 96`. Many desktops leave that at
//! 96 even on high-DPI monitors, so everything comes out tiny. This works
//! out the scale the monitor's physical size calls for instead. On Wayland
//! the compositor already scales windows, so nothing is added there.

use std::env;
use std::process::Command;
use std::thread;

/// Monitors reporting a smaller width (projectors, some TVs, broken EDIDs)
/// can't be trusted.
const MIN_WIDTH_MM: f64 = 100.0;
const MAX_AUTO_SCALE: f64 = 3.0;

/// The extra scale revmo should apply on this display, 1.0 if none.
pub fn auto_scale() -> f32 {
    if !on_x11_unscaled(|key| env::var_os(key).filter(|v| !v.is_empty()).is_some()) {
        return 1.0;
    }
    // Both take a few milliseconds; run them side by side.
    let xft = thread::spawn(|| run("xrdb", &["-query"]));
    let monitors = run("xrandr", &["--current"]);
    let xft = xft.join().ok().flatten();
    match (monitors, xft) {
        (Some(monitors), Some(xft)) => scale_for(&monitors, &xft),
        _ => 1.0,
    }
}

/// Whether revmo will run on X11 (the way winit decides: Wayland when either
/// of its variables is non-empty) without the user overriding the scale.
/// `set` tells whether an environment variable is set and non-empty.
fn on_x11_unscaled(set: impl Fn(&str) -> bool) -> bool {
    !set("WAYLAND_DISPLAY")
        && !set("WAYLAND_SOCKET")
        && set("DISPLAY")
        && !set("WINIT_X11_SCALE_FACTOR")
}

fn run(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The scale for the primary monitor (or the first active one) in `xrandr
/// --current` output, given
/// `xrdb -query` output. Without an `Xft.dpi`, X11 apps already scale by the
/// monitor's size themselves, so this adds nothing.
fn scale_for(xrandr: &str, xrdb: &str) -> f32 {
    let Some(xft_dpi) = xrdb.lines().find_map(|l| {
        let value = l.strip_prefix("Xft.dpi:")?;
        value.trim().parse::<f64>().ok().filter(|d| *d > 0.0)
    }) else {
        return 1.0;
    };
    let Some(dpi) = primary_dpi(xrandr) else {
        return 1.0;
    };
    // Round to quarters, which keep 1px lines crisp.
    let scale = (dpi / xft_dpi * 4.0).round() / 4.0;
    scale.clamp(1.0, MAX_AUTO_SCALE) as f32
}

/// Physical dpi of the primary monitor (or the first active one), measured
/// on the diagonal so rotation doesn't matter.
fn primary_dpi(xrandr: &str) -> Option<f64> {
    let active: Vec<(f64, bool)> = xrandr
        .lines()
        .filter(|l| l.contains(" connected"))
        .filter_map(|l| Some((monitor_dpi(l)?, l.contains(" connected primary "))))
        .collect();
    active
        .iter()
        .find(|(_, primary)| *primary)
        .or(active.first())
        .map(|(dpi, _)| *dpi)
}

/// From a line like
/// `DP-1 connected primary 3840x2160+0+0 (normal left) 697mm x 392mm`.
fn monitor_dpi(line: &str) -> Option<f64> {
    let words: Vec<&str> = line.split_whitespace().collect();
    // The mode is the first WxH+X+Y word; inactive monitors have none.
    let (px_w, px_h) = words.iter().find_map(|w| {
        let (size, _) = w.split_once('+')?;
        let (width, height) = size.split_once('x')?;
        Some((width.parse::<f64>().ok()?, height.parse::<f64>().ok()?))
    })?;
    let mm = |w: &&str| w.strip_suffix("mm")?.parse::<f64>().ok();
    let mut sizes = words.iter().filter_map(mm);
    let (mm_w, mm_h) = (sizes.next()?, sizes.next()?);
    if mm_w.max(mm_h) < MIN_WIDTH_MM {
        return None;
    }
    let pixels = px_w.hypot(px_h);
    let inches = mm_w.hypot(mm_h) / 25.4;
    Some(pixels / inches)
}

#[cfg(test)]
mod tests {
    use super::*;

    const XRANDR: &str = "\
Screen 0: minimum 8 x 8, current 3840 x 2160, maximum 32767 x 32767
eDP-1 connected (normal left inverted right x axis y axis)
DP-1-1 connected primary 3840x2160+0+0 (normal left inverted right x axis y axis) 697mm x 392mm
   3840x2160     60.00*+  30.00
";

    #[test]
    fn only_scales_plain_x11() {
        let env = |vars: &'static [&'static str]| move |key: &str| vars.contains(&key);
        assert!(on_x11_unscaled(env(&["DISPLAY"])));
        assert!(!on_x11_unscaled(env(&["DISPLAY", "WAYLAND_DISPLAY"])));
        assert!(!on_x11_unscaled(env(&["DISPLAY", "WAYLAND_SOCKET"])));
        assert!(!on_x11_unscaled(env(&[
            "DISPLAY",
            "WINIT_X11_SCALE_FACTOR"
        ])));
        assert!(!on_x11_unscaled(env(&[])));
    }

    #[test]
    fn scales_4k_27_inch_at_96_dpi() {
        // 3840 px over 697 mm is about 140 dpi: 1.46, rounded to 1.5.
        assert_eq!(scale_for(XRANDR, "Xft.dpi:\t96\n"), 1.5);
        // The desktop already scales by 1.5: nothing more to add.
        assert_eq!(scale_for(XRANDR, "Xft.dpi:\t144\n"), 1.0);
        // No Xft.dpi: X11 apps already scale by the monitor.
        assert_eq!(scale_for(XRANDR, "Xcursor.size: 24\n"), 1.0);
    }

    #[test]
    fn picks_primary_and_ignores_bad_sizes() {
        let two = "\
HDMI-1 connected 1920x1080+3840+0 (normal) 527mm x 296mm
DP-1 connected primary 3840x2160+0+0 (normal) 697mm x 392mm
";
        assert_eq!(scale_for(two, "Xft.dpi: 96"), 1.5);
        let no_primary = "HDMI-1 connected 1920x1080+0+0 (normal) 527mm x 296mm\n";
        assert_eq!(scale_for(no_primary, "Xft.dpi: 96"), 1.0);
        // A rotated monitor gives the same dpi.
        let rotated = "DP-1 connected primary 2160x3840+0+0 left (normal) 697mm x 392mm\n";
        assert_eq!(scale_for(rotated, "Xft.dpi: 96"), 1.5);
        // Projectors report no or tiny sizes.
        let projector = "VGA-1 connected primary 1024x768+0+0 (normal) 0mm x 0mm\n";
        assert_eq!(scale_for(projector, "Xft.dpi: 96"), 1.0);
        assert_eq!(scale_for("", "Xft.dpi: 96"), 1.0);
        // A tiny, very dense screen still stops at the maximum.
        let dense = "DP-1 connected primary 7680x4320+0+0 (normal) 120mm x 68mm\n";
        assert_eq!(scale_for(dense, "Xft.dpi: 96"), 3.0);
    }
}
