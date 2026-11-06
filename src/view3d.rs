#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View3D {
    pub cell: f32,
    pub pan: (f32, f32),
    pub rot_deg: f32,
    pub tilt_deg: f32,
    pub hf: f32,
    pub e_off: f32,
    pub e_scale: f32,
}

impl Default for View3D {
    fn default() -> Self {
        Self {
            cell: 14.0,
            pan: (0.0, 0.0),
            rot_deg: 0.0,
            tilt_deg: 60.0,
            hf: 1.0,
            e_off: 0.0,
            e_scale: 1.0,
        }
    }
}

impl View3D {
    pub fn disp_e(&self, e: f32) -> f32 {
        let d = e - self.e_off;
        let v = if d >= 0.0 {
            d / (1.0 - self.e_off).max(0.01) * self.e_scale
        } else {
            d * self.e_scale
        };
        v.clamp(0.0, 64.0)
    }

    pub fn project(&self, x: f32, y: f32, e: f32, cx: f32, cy: f32, center: (f32, f32)) -> (f32, f32) {
        let e = self.disp_e(e);
        let a = self.tilt_deg.to_radians();
        let (sa, ca) = (a.sin(), a.cos());
        let dx = x - cx;
        let dy = y - cy;
        let r = self.rot_deg.to_radians();
        let (s, c) = (r.sin(), r.cos());
        let rx = dx * c - dy * s;
        let ry = dx * s + dy * c;
        let tx = rx + e * 0.5 * sa * ca;
        let ty = ry * sa - e * self.hf * ca;
        (
            center.0 + self.pan.0 + tx * self.cell,
            center.1 + self.pan.1 + ty * self.cell,
        )
    }

    pub fn unproject(&self, p: (f32, f32), cx: f32, cy: f32, center: (f32, f32), e_hint: f32) -> (f32, f32) {
        let vx = (p.0 - center.0 - self.pan.0) / self.cell;
        let vy = (p.1 - center.1 - self.pan.1) / self.cell;
        let a = self.tilt_deg.to_radians();
        let (sa, ca) = (a.sin().max(0.17), a.cos());
        let e = self.disp_e(e_hint);
        let rx = vx - e * 0.5 * sa * ca;
        let ry = (vy + e * self.hf * ca) / sa;
        let r = self.rot_deg.to_radians();
        let (s, c) = (r.sin(), r.cos());
        (
            cx + rx * c + ry * s,
            cy + -rx * s + ry * c,
        )
    }

    pub fn rotate_at(&mut self, p: (f32, f32), cx: f32, cy: f32, center: (f32, f32), delta_deg: f32) {
        let (ax, ay) = self.unproject(p, cx, cy, center, self.e_off);
        self.rot_deg += delta_deg;
        let sp = self.project(ax, ay, self.e_off, cx, cy, center);
        self.pan.0 += p.0 - sp.0;
        self.pan.1 += p.1 - sp.1;
    }

    pub fn zoom_at(&mut self, p: (f32, f32), anchor_screen: (f32, f32), center: (f32, f32), factor: f32) {
        let new_cell = (self.cell * factor).clamp(4.0, 40.0);
        let f = new_cell / self.cell;
        self.pan.0 = (p.0 - center.0) - (anchor_screen.0 - center.0 - self.pan.0) * f;
        self.pan.1 = (p.1 - center.1) - (anchor_screen.1 - center.1 - self.pan.1) * f;
        self.cell = new_cell;
        if !self.pan.0.is_finite() || !self.pan.1.is_finite() || !self.cell.is_finite() {
            self.pan = (0.0, 0.0);
            self.cell = 14.0;
        }
    }
}
