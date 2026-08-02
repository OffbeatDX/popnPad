pub const KEY_COUNT: usize = 11;

pub const POP_L: usize = 9;
pub const POP_R: usize = 10;

pub const BUTTON_NAMES: [&str; KEY_COUNT] = [
    "Button 1",
    "Button 2",
    "Button 3",
    "Button 4",
    "Button 5",
    "Button 6",
    "Button 7",
    "Button 8",
    "Button 9",
    "Red Pop-Kun",
    "Blue Pop-Kun",
];

const PITCH: f32 = 0.62;
const ROW_DY: f32 = 0.78;
const FLAT_PITCH: f32 = 1.0;
const POP_RADIUS: f32 = 0.39;
const POP_ROW_GAP: f32 = 0.12;

const BAND_PAD: f32 = 0.18;
const CHROME_H: f32 = 60.0;
const MIN_FIT: f32 = 0.4;
const HOLD_SLOP: f32 = 1.3;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Disc {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
}

impl Disc {
    fn valid(&self) -> bool {
        self.r > 1.0
    }

    fn dist2(&self, x: f32, y: f32) -> f32 {
        let (dx, dy) = (x - self.cx, y - self.cy);
        dx * dx + dy * dy
    }

    fn covers(&self, x: f32, y: f32, slop: f32) -> bool {
        let reach = self.r * slop;
        self.valid() && self.dist2(x, y) <= reach * reach
    }
}

#[derive(Debug, Clone, Copy)]
struct Metrics {
    pitch: f32,
    row_dy: f32,
    single_row: bool,
}

impl Metrics {
    fn of(single_row: bool) -> Self {
        if single_row {
            Metrics {
                pitch: FLAT_PITCH,
                row_dy: 0.0,
                single_row,
            }
        } else {
            Metrics {
                pitch: PITCH,
                row_dy: ROW_DY,
                single_row,
            }
        }
    }

    fn span_w(&self) -> f32 {
        if self.single_row {
            8.0 * self.pitch + 1.0
        } else {
            10.0 * self.pitch + 2.0 * POP_RADIUS
        }
    }

    fn span_h(&self) -> f32 {
        if self.single_row {
            2.0 * POP_RADIUS + POP_ROW_GAP + 1.0
        } else {
            self.row_dy + 1.0
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Band {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Band {
    pub fn valid(&self) -> bool {
        self.w > 1.0 && self.h > 1.0
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.valid() && x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyGeom {
    pub band: Band,
    pub keys: [Disc; KEY_COUNT],
}

impl Default for KeyGeom {
    fn default() -> Self {
        KeyGeom {
            band: Band::default(),
            keys: [Disc::default(); KEY_COUNT],
        }
    }
}

impl KeyGeom {
    pub fn layout(panel: Band, fit: f32, size: f32, pos: f32, single_row: bool) -> Self {
        if !panel.valid() {
            return Self::default();
        }
        let m = Metrics::of(single_row);
        let content_w = panel.w * (MIN_FIT + (1.0 - MIN_FIT) * fit.clamp(0.0, 1.0));
        let margin = (content_w * 0.01).max(12.0);
        let avail_w = content_w - 2.0 * margin;
        let avail_h = panel.h - CHROME_H - margin;
        if avail_w <= 0.0 || avail_h <= 0.0 {
            return Self::default();
        }

        let d = (avail_w / m.span_w()).min(avail_h / m.span_h()) * size.clamp(0.2, 1.0);
        if d * POP_RADIUS <= 1.0 {
            return Self::default();
        }
        let r = d * 0.5;

        let span_w = d * m.span_w();
        let span_h = d * m.span_h();
        let left = panel.x + (panel.w - span_w) * 0.5;
        let free_h = (avail_h - span_h).max(0.0);
        let top = panel.y + CHROME_H + pos.clamp(0.0, 1.0) * free_h;

        let pitch = d * m.pitch;
        let pop_r = d * POP_RADIUS;

        let (x0, upper_cy, lower_cy, pop_cy) = if m.single_row {
            let row_cy = top + span_h - r;
            (left + r, row_cy, row_cy, top + pop_r)
        } else {
            let top_cy = top + r;
            (left + pop_r + pitch, top_cy, top + d * m.row_dy + r, top_cy)
        };

        let mut keys = [Disc::default(); KEY_COUNT];
        for (i, key) in keys.iter_mut().take(9).enumerate() {
            let upper = i % 2 == 1;
            *key = Disc {
                cx: x0 + i as f32 * pitch,
                cy: if upper { upper_cy } else { lower_cy },
                r,
            };
        }

        let (pop_l_cx, pop_r_cx) = if m.single_row {
            (x0, x0 + 8.0 * pitch)
        } else {
            (x0 - pitch, x0 + 9.0 * pitch)
        };
        keys[POP_L] = Disc {
            cx: pop_l_cx,
            cy: pop_cy,
            r: pop_r,
        };
        keys[POP_R] = Disc {
            cx: pop_r_cx,
            cy: pop_cy,
            r: pop_r,
        };

        let pad = d * BAND_PAD;
        let band_y = (top - pad).max(panel.y + CHROME_H);
        KeyGeom {
            band: Band {
                x: left - pad,
                y: band_y,
                w: span_w + 2.0 * pad,
                h: top + span_h + pad - band_y,
            },
            keys,
        }
    }

    pub fn offset_y(mut self, dy: f32) -> Self {
        self.band.y += dy;
        for k in &mut self.keys {
            k.cy += dy;
        }
        self
    }

    pub fn valid(&self) -> bool {
        self.band.valid() && self.keys.iter().all(Disc::valid)
    }

    pub fn in_band(&self, x: f32, y: f32) -> bool {
        self.band.contains(x, y)
    }

    fn landed(&self, x: f32, y: f32) -> Option<usize> {
        self.keys
            .iter()
            .enumerate()
            .filter(|(_, d)| d.covers(x, y, 1.0))
            .min_by(|(_, a), (_, b)| a.dist2(x, y).total_cmp(&b.dist2(x, y)))
            .map(|(i, _)| i)
    }

    pub fn hit(&self, x: f32, y: f32, prev: Option<usize>) -> Option<usize> {
        if !self.valid() {
            return None;
        }
        let landed = self.landed(x, y);
        match prev.filter(|p| *p < KEY_COUNT) {
            Some(p) if landed.is_none() && self.keys[p].covers(x, y, HOLD_SLOP) => Some(p),
            _ => landed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Band {
        Band {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        }
    }

    fn geom() -> KeyGeom {
        KeyGeom::layout(panel(), 1.0, 1.0, 0.5, false)
    }

    fn flat() -> KeyGeom {
        KeyGeom::layout(panel(), 1.0, 1.0, 0.5, true)
    }

    #[test]
    fn the_layout_places_nine_buttons_and_two_pop_kuns() {
        let g = geom();
        assert!(g.valid());

        let odd: Vec<f32> = [0, 2, 4, 6, 8].iter().map(|i| g.keys[*i].cy).collect();
        let even: Vec<f32> = [1, 3, 5, 7].iter().map(|i| g.keys[*i].cy).collect();
        assert!(
            odd.windows(2).all(|w| w[0] == w[1]),
            "buttons 1 3 5 7 9 share the lower row: {odd:?}"
        );
        assert!(
            even.windows(2).all(|w| w[0] == w[1]),
            "buttons 2 4 6 8 share the upper row: {even:?}"
        );
        assert!(even[0] < odd[0], "the even row sits above the odd row");

        let xs: Vec<f32> = (0..9).map(|i| g.keys[i].cx).collect();
        assert!(
            xs.windows(2).all(|w| w[0] < w[1]),
            "1..9 run left to right: {xs:?}"
        );

        assert!(
            g.keys[POP_L].cx < g.keys[0].cx,
            "the red Pop-Kun sits left of button 1"
        );
        assert!(
            g.keys[POP_R].cx > g.keys[8].cx,
            "the blue Pop-Kun sits right of button 9"
        );
        assert_eq!(
            g.keys[POP_L].cy, g.keys[1].cy,
            "Pop-Kuns ride the upper row"
        );
        assert_eq!(g.keys[POP_L].r, g.keys[POP_R].r);
        assert!(
            g.keys[POP_L].r < g.keys[0].r,
            "Pop-Kuns are the smaller pair"
        );
    }

    #[test]
    fn adjacent_buttons_touch_but_do_not_swallow_each_other() {
        let g = geom();
        let d = g.keys[0].r * 2.0;
        for i in 0..8 {
            let gap = g.keys[i].dist2(g.keys[i + 1].cx, g.keys[i + 1].cy).sqrt();
            assert!(
                (gap - d).abs() < d * 0.02,
                "buttons {} and {} should just touch: centres {gap:.1} apart, diameter {d:.1}",
                i + 1,
                i + 2
            );
        }
    }

    #[test]
    fn every_button_centre_hits_its_own_index() {
        let g = geom();
        for i in 0..KEY_COUNT {
            let k = g.keys[i];
            assert_eq!(g.hit(k.cx, k.cy, None), Some(i), "centre of key {i}");
        }
    }

    #[test]
    fn a_point_outside_every_disc_hits_nothing() {
        let g = geom();
        assert_eq!(g.hit(g.band.x - 50.0, g.keys[0].cy, None), None);
        assert_eq!(g.hit(g.keys[0].cx, g.band.y + g.band.h + 50.0, None), None);
    }

    #[test]
    fn the_gap_between_same_row_buttons_holds_the_key_it_came_from() {
        let g = geom();
        let mid = (g.keys[0].cx + g.keys[2].cx) * 0.5;
        let y = g.keys[0].cy;
        assert_eq!(
            g.hit(mid, y, None),
            None,
            "a fresh contact in the gap idles"
        );
        assert_eq!(
            g.hit(mid, y, Some(0)),
            Some(0),
            "but a finger already on button 1 keeps it across the gap"
        );
    }

    #[test]
    fn sliding_onto_the_next_button_retargets() {
        let g = geom();
        let k = g.keys[1];
        assert_eq!(
            g.hit(k.cx, k.cy, Some(0)),
            Some(1),
            "landing inside button 2 wins over holding button 1"
        );
    }

    #[test]
    fn the_band_covers_the_buttons_but_starts_below_the_chrome_strip() {
        let g = geom();
        assert!(g.in_band(g.keys[0].cx, g.keys[0].cy));
        assert!(g.in_band(g.keys[POP_R].cx, g.keys[POP_R].cy));
        assert!(
            g.band.y >= CHROME_H,
            "the band must never reach the gear/NAV/close row"
        );
        assert!(!g.in_band(20.0, 20.0), "the top strip stays chrome");
    }

    #[test]
    fn the_panel_position_slides_the_cluster_without_resizing_it() {
        let high = KeyGeom::layout(panel(), 1.0, 1.0, 0.0, false);
        let low = KeyGeom::layout(panel(), 1.0, 1.0, 1.0, false);
        assert_eq!(high.keys[0].r, low.keys[0].r);
        assert!(low.keys[0].cy > high.keys[0].cy);
        assert_eq!(high.keys[0].cx, low.keys[0].cx);
    }

    #[test]
    fn button_size_shrinks_the_cluster_and_keeps_it_centred() {
        let full = geom();
        let small = KeyGeom::layout(panel(), 1.0, 0.5, 0.5, false);
        assert!(small.keys[0].r < full.keys[0].r);
        let centre = |g: &KeyGeom| (g.keys[POP_L].cx + g.keys[POP_R].cx) * 0.5;
        assert!((centre(&full) - centre(&small)).abs() < 1.0);
    }

    #[test]
    fn a_panel_with_no_room_yields_nothing_hittable() {
        let tiny = Band {
            x: 0.0,
            y: 0.0,
            w: 40.0,
            h: 40.0,
        };
        let g = KeyGeom::layout(tiny, 1.0, 1.0, 0.5, false);
        assert!(!g.valid());
        assert_eq!(g.hit(20.0, 20.0, None), None);
    }

    #[test]
    fn single_row_lines_the_nine_up_and_keeps_them_touching() {
        let g = flat();
        assert!(g.valid());

        let cy = g.keys[0].cy;
        assert!(
            g.keys[..9].iter().all(|k| k.cy == cy),
            "all nine buttons share one row: {:?}",
            g.keys[..9].iter().map(|k| k.cy).collect::<Vec<_>>()
        );

        let d = g.keys[0].r * 2.0;
        for i in 0..8 {
            let step = g.keys[i + 1].cx - g.keys[i].cx;
            assert!(
                (step - d).abs() < d * 0.02,
                "buttons {} and {} should sit tangent: {step:.1} apart, diameter {d:.1}",
                i + 1,
                i + 2
            );
        }
    }

    #[test]
    fn single_row_lifts_the_pop_kuns_clear_so_the_nine_own_the_full_width() {
        let g = flat();

        assert!(
            g.keys[POP_L].cy < g.keys[0].cy && g.keys[POP_R].cy < g.keys[8].cy,
            "the Pop-Kuns take their own row above the nine"
        );
        assert_eq!(g.keys[POP_L].cy, g.keys[POP_R].cy);

        let left = g.keys[0].cx - g.keys[0].r;
        let right = g.keys[8].cx + g.keys[8].r;
        for i in [POP_L, POP_R] {
            assert!(
                g.keys[i].cx - g.keys[i].r >= left - 0.5,
                "Pop-Kun {i} must not widen the panel past button 1"
            );
            assert!(
                g.keys[i].cx + g.keys[i].r <= right + 0.5,
                "Pop-Kun {i} must not widen the panel past button 9"
            );
        }

        let used = right - left;
        assert!(
            (used - 9.0 * g.keys[0].r * 2.0).abs() < 1.0,
            "the nine sit shoulder to shoulder across nine full diameters: {used:.1}"
        );
    }

    #[test]
    fn the_pop_kuns_join_the_stagger_rather_than_sitting_in_a_gutter() {
        let g = geom();
        let pitch = g.keys[1].cx - g.keys[0].cx;

        assert!(
            (g.keys[0].cx - g.keys[POP_L].cx - pitch).abs() < 1.0,
            "the red Pop-Kun takes the lattice slot one pitch left of button 1"
        );
        assert!(
            (g.keys[POP_R].cx - g.keys[8].cx - pitch).abs() < 1.0,
            "the blue Pop-Kun takes the slot one pitch right of button 9"
        );

        let d = g.keys[0].r * 2.0;
        let reach = g.keys[POP_L].r + g.keys[0].r;
        let gap = g.keys[POP_L].dist2(g.keys[0].cx, g.keys[0].cy).sqrt();
        assert!(
            gap > reach && gap < reach + d * 0.2,
            "it should tuck in close to button 1 without overlapping: {gap:.1} vs {reach:.1}"
        );
    }

    #[test]
    fn single_row_hits_every_button_at_its_centre() {
        let g = flat();
        for i in 0..KEY_COUNT {
            let k = g.keys[i];
            assert_eq!(g.hit(k.cx, k.cy, None), Some(i), "centre of key {i}");
        }
    }

    #[test]
    fn single_row_trades_button_size_for_a_shorter_panel() {
        let staggered = geom();
        let g = flat();
        assert!(
            g.keys[0].r < staggered.keys[0].r,
            "eleven discs abreast need a wider span, so they come out smaller"
        );
        assert!(
            g.band.h < staggered.band.h,
            "but the panel is only one row tall"
        );
    }

    #[test]
    fn button_names_line_up_with_the_spice2x_order() {
        assert_eq!(BUTTON_NAMES[0], "Button 1");
        assert_eq!(BUTTON_NAMES[8], "Button 9");
        assert_eq!(BUTTON_NAMES[POP_L], "Red Pop-Kun");
        assert_eq!(BUTTON_NAMES[POP_R], "Blue Pop-Kun");
    }
}
