use std::cmp::Ordering;
use std::collections::LinkedList;
use std::f32::consts::FRAC_1_SQRT_2 as SCALE;
use std::fmt::{self, Write};

pub trait Draw {
    type Output;

    fn line(&mut self, x: f32, y: f32) -> Self::Output;
    fn horiz(&mut self, x: f32) -> Self::Output {
        self.line(x, 0.0)
    }
    fn vert(&mut self, y: f32) -> Self::Output {
        self.line(0.0, y)
    }
}

pub struct SvgPath<'a> {
    /// Underlying writer to write to
    pub writer: &'a mut dyn Write,
}
impl Draw for SvgPath<'_> {
    type Output = fmt::Result;

    fn line(&mut self, x: f32, y: f32) -> fmt::Result {
        write!(self.writer, "l{x} {y}")
    }
    fn horiz(&mut self, x: f32) -> fmt::Result {
        write!(self.writer, "h{x}")
    }
    fn vert(&mut self, y: f32) -> fmt::Result {
        write!(self.writer, "v{y}")
    }
}

#[allow(dead_code)] // variants are constructed through transmutes
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Dir {
    Npp, // ++
    Np0, // +0
    Npm, // +-
    N0m, // 0-
    Nmm, // --
    Nm0, // -0
    Nmp, // -+
    N0p, // 0+
}
impl Dir {
    #[inline(always)]
    pub const unsafe fn from_u8_unchecked(idx: u8) -> Self {
        unsafe { std::mem::transmute(idx) }
    }
    #[inline(always)]
    pub const fn from_u8(idx: u8) -> Self {
        unsafe { Self::from_u8_unchecked(idx % 8) }
    }
    #[inline(always)]
    pub fn rotate(&self, by: u8) -> Self {
        Self::from_u8(*self as u8 + by)
    }
    #[inline(always)]
    pub fn right(&self) -> Self {
        self.rotate(1)
    }
    #[inline(always)]
    pub fn left(&self) -> Self {
        self.rotate(7)
    }
    pub fn draw<D: Draw>(&self, out: &mut D, len: f32) -> D::Output {
        match self {
            Self::Npp => out.line(len * SCALE, len * SCALE),
            Self::Np0 => out.horiz(len),
            Self::Npm => out.line(len * SCALE, len * -SCALE),
            Self::N0m => out.vert(-len),
            Self::Nmm => out.line(len * -SCALE, len * -SCALE),
            Self::Nm0 => out.horiz(-len),
            Self::Nmp => out.line(len * -SCALE, len * SCALE),
            Self::N0p => out.vert(len),
        }
    }
    fn write_svg(&self, w: &mut dyn Write, scale: f32) -> fmt::Result {
        match self {
            Self::Npp => write!(w, " l{} {}", scale * SCALE, scale * SCALE),
            Self::Np0 => write!(w, " h{}", scale),
            Self::Npm => write!(w, " l{} {}", scale * SCALE, scale * -SCALE),
            Self::N0m => write!(w, " v{}", -scale),
            Self::Nmm => write!(w, " l{} {}", scale * -SCALE, scale * -SCALE),
            Self::Nm0 => write!(w, " h{}", -scale),
            Self::Nmp => write!(w, " l{} {}", scale * -SCALE, scale * SCALE),
            Self::N0p => write!(w, " v{}", scale),
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CurveFlags: u8 {
        const DRAGON = 0b00;
        const NONE = 0b00;
        const LEVY = 0b01;
        const FLIP = 0b10;
    }
}

#[derive(Debug, Clone)]
pub struct DragonCurve {
    list: LinkedList<Dir>,
    depth: u8,
    flags: CurveFlags,
}
impl DragonCurve {
    pub fn new(start: Dir, flags: CurveFlags) -> Self {
        Self {
            list: LinkedList::from([start]),
            depth: 0,
            flags,
        }
    }
    pub fn rotate_by(&mut self, by: u8) {
        for elem in &mut self.list {
            *elem = elem.rotate(by);
        }
    }
    pub fn rotate_to(&mut self, to: Dir) {
        let by = *self.list.front().unwrap() as u8 + 8 - to as u8;
        self.rotate_by(by);
    }
    pub fn set_depth(&mut self, depth: u8) {
        match self.depth.cmp(&depth) {
            Ordering::Equal => {}
            Ordering::Less => {
                for _ in self.depth..depth {
                    let mut cursor = self.list.cursor_front_mut();
                    let mut right = true;
                    while let Some(dir) = cursor.current() {
                        let new = if (self.flags.contains(CurveFlags::LEVY) || right)
                            ^ self.flags.contains(CurveFlags::FLIP)
                        {
                            let d2 = dir.right();
                            *dir = dir.left();
                            d2
                        } else {
                            let d2 = dir.left();
                            *dir = dir.right();
                            d2
                        };
                        cursor.move_next();
                        cursor.insert_before(new);
                        right = !right;
                    }
                }
            }
            Ordering::Greater => {
                if depth == 0 {
                    let mut cursor = self.list.cursor_front_mut();
                    let front = cursor.current().unwrap();
                    *front = front.rotate(if self.flags.contains(CurveFlags::FLIP) { 8u8.wrapping_sub(self.depth) } else { self.depth });
                    cursor.split_after();
                } else {
                    let new_len = 1 << depth;
                    let mut idx = 0;
                    let mut rot = self.depth - depth;
                    if self.flags.contains(CurveFlags::FLIP) {
                        rot = 8u8.wrapping_sub(rot);
                    }
                    let mut cursor = self.list.cursor_front_mut();
                    while let Some(elem) = cursor.current() {
                        *elem = elem.rotate(rot);
                        idx += 1;
                        if idx == new_len {
                            cursor.split_after();
                            break;
                        } else {
                            cursor.move_next();
                        }
                    }
                }
            }
        }
        self.depth = depth;
    }
    /// Depth of the curve
    pub fn depth(&self) -> u8 {
        self.depth
    }
    /// Length of the curve, not including the final point
    pub fn len(&self) -> usize {
        self.list.len()
    }
    pub fn list(&self) -> &LinkedList<Dir> {
        &self.list
    }
    pub fn flags(&self) -> CurveFlags {
        self.flags
    }
    pub fn write_svg(&self, size: f32, w: &mut dyn Write) -> fmt::Result {
        let mut step = size / (1 << (self.depth / 2) + 1) as f32;
        if self.depth & 1 != 0 {
            step *= SCALE;
        }
        let start = format!("{} {}", size * 0.25, size * 0.5);
        write!(
            w,
            r#"<svg width="{size}" height="{size}" xmlns="http://www.w3.org/2000/svg"><path style="stroke:black;stroke-width:1;fill:none" d="M{start}"#
        )?;
        for p in &self.list {
            p.write_svg(w, step)?;
        }
        write!(w, r#" M{start}"/></svg>"#)
    }
}
impl PartialEq for DragonCurve {
    fn eq(&self, other: &Self) -> bool {
        self.depth == other.depth && self.flags == other.flags
    }
}
