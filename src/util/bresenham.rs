use glam::IVec3;

pub struct Bresenham {
    current: IVec3,
    end: IVec3,
    step: IVec3,
    dx: i32,
    dy: i32,
    dz: i32,
    dominant: u8,
    err1: i32,
    err2: i32,
    finished: bool,
}

impl Bresenham {
    pub fn new(start: IVec3, end: IVec3) -> Self {
        let delta = end - start;
        let abs_delta = delta.abs();
        let step = delta.signum();

        let dx = abs_delta.x;
        let dy = abs_delta.y;
        let dz = abs_delta.z;

        let dominant = if dx >= dy && dx >= dz {
            0
        } else if dy >= dx && dy >= dz {
            1
        } else {
            2
        };

        let (err1, err2) = match dominant {
            0 => (2 * dy - dx, 2 * dz - dx),
            1 => (2 * dx - dy, 2 * dz - dy),
            _ => (2 * dy - dz, 2 * dx - dz),
        };

        Self {
            current: start,
            end,
            step,
            dx,
            dy,
            dz,
            dominant,
            err1,
            err2,
            finished: false,
        }
    }
}

impl Iterator for Bresenham {
    type Item = IVec3;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let result = self.current;

        if self.current == self.end {
            self.finished = true;
            return Some(result);
        }

        match self.dominant {
            0 => {
                if self.err1 >= 0 {
                    self.current.y += self.step.y;
                    self.err1 -= 2 * self.dx;
                }
                if self.err2 >= 0 {
                    self.current.z += self.step.z;
                    self.err2 -= 2 * self.dx;
                }
                self.err1 += 2 * self.dy;
                self.err2 += 2 * self.dz;
                self.current.x += self.step.x;
            }
            1 => {
                if self.err1 >= 0 {
                    self.current.x += self.step.x;
                    self.err1 -= 2 * self.dy;
                }
                if self.err2 >= 0 {
                    self.current.z += self.step.z;
                    self.err2 -= 2 * self.dy;
                }
                self.err1 += 2 * self.dx;
                self.err2 += 2 * self.dz;
                self.current.y += self.step.y;
            }
            _ => {
                if self.err1 >= 0 {
                    self.current.y += self.step.y;
                    self.err1 -= 2 * self.dz;
                }
                if self.err2 >= 0 {
                    self.current.x += self.step.x;
                    self.err2 -= 2 * self.dz;
                }
                self.err1 += 2 * self.dy;
                self.err2 += 2 * self.dx;
                self.current.z += self.step.z;
            }
        }

        Some(result)
    }
}
