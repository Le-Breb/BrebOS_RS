#![allow(dead_code)]

use crate::framebuffer::FbColor::{Black, White};
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::framebuffer;

pub const FB_WIDTH: usize = 80;
pub const FB_HEIGHT: usize = 25;
pub const VGA_MEM: *mut u8 = 0xB8000 as *mut u8;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::framebuffer::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub fn _print(args: core::fmt::Arguments)
{
    use x86_64::instructions::interrupts;
    use core::fmt::Write;

    interrupts::without_interrupts(|| {
        framebuffer::FB.lock().write_fmt(args).unwrap()
    });
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FbColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: FbColor, background: FbColor) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

lazy_static! {
    pub static ref FB: Mutex<Framebuffer> = Mutex::new(Framebuffer::new());
}

#[derive(Copy, Clone)]
pub struct Cell {
    pub c: char,
    color_code: ColorCode,
}

impl From<ColorCode> for u8 {
    fn from(c: ColorCode) -> Self {
        c.0
    }
}

impl Cell {
    pub fn new(c: char, bg: FbColor, fg: FbColor) -> Self {
        Cell {
            c,
            color_code: ColorCode::new(fg, bg),
        }
    }
}

struct DirtyRect {
    start_x: usize,
    start_y: usize,
    end_x: usize,
    end_y: usize,
}

impl DirtyRect {
    fn new() -> Self {
        DirtyRect {
            start_x: 0,
            start_y: 0,
            end_x: 0,
            end_y: 0,
        }
    }

    fn register_updated_char(&mut self, pos_x: usize, pos_y: usize) {
        if pos_x < self.start_x {
            self.start_x = pos_x;
        }
        if pos_x > self.end_x {
            self.end_x = pos_x;
        }
        if pos_y < self.start_y {
            self.start_y = pos_y;
        }
        if pos_y > self.end_y {
            self.end_y = pos_y;
        }
    }

    fn get_start_x(&self) -> usize {
        self.start_x
    }
    fn get_start_y(&self) -> usize {
        self.start_y
    }

    fn get_end_x(&self) -> usize {
        self.end_x
    }
    fn get_end_y(&self) -> usize {
        self.end_x
    }

    fn mark_all_screen_dirty(&mut self)
    {
        self.start_x = 0;
        self.start_y = 0;
        self.end_x = FB_WIDTH - 1;
        self.end_y = FB_HEIGHT - 1;
    }

    fn reset(&mut self) {
        self.start_x = 0;
        self.start_y = 0;
        self.end_x = 0;
        self.end_y = 0;
    }
}

pub struct Framebuffer {
    caret_pos_x: usize,
    caret_pos_y: usize,
    bg_color: FbColor,
    fg_color: FbColor,
    buf: [Cell; FB_WIDTH * FB_HEIGHT],
    dirty_rect: DirtyRect,
}

impl Framebuffer {
    pub fn new() -> Self {
        Framebuffer {
            caret_pos_x: 0,
            caret_pos_y: 0,
            bg_color: FbColor::Black,
            fg_color: FbColor::White,
            buf: [Cell::new(' ', FbColor::Black, FbColor::White); FB_WIDTH * FB_HEIGHT],
            dirty_rect: DirtyRect::new(),
        }
    }

    pub fn write_str(&mut self, string: &str) {
        for c in string.chars() {
            self.write_char(c);
        }
    }

    pub fn clear_screen(&mut self) {
        self.caret_pos_x = 0;
        self.caret_pos_y = 0;

        let blank_cell: Cell = Cell::new(' ', Black, White);

        for idx in 0..FB_WIDTH * FB_HEIGHT {
            self.buf[idx] = blank_cell;
        }

        self.dirty_rect.mark_all_screen_dirty();
        self.flush();
    }

    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.caret_pos_y += 1;
                if self.caret_pos_y == FB_HEIGHT {
                    self.scroll();
                    self.caret_pos_y -= 1;
                }
                self.caret_pos_x = 0;
                self.flush();
            }
            _ => self.write_cell(Cell::new(c, self.bg_color, self.fg_color))
        }
    }

    #[allow(dead_code)]
    pub fn set_fg_color(&mut self, color: FbColor) {
        self.fg_color = color;
    }

    #[allow(dead_code)]
    pub fn set_bg_color(&mut self, color: FbColor) {
        self.bg_color = color;
    }

    fn write_cell(&mut self, cell: Cell) {
        self.dirty_rect.register_updated_char(self.caret_pos_x, self.caret_pos_y);

        let pos: usize = self.caret_pos_y * FB_WIDTH + self.caret_pos_x;
        self.buf[pos] = cell;

        let mut scrolled = false;

        self.caret_pos_x += 1;
        if self.caret_pos_x == FB_WIDTH {
            self.caret_pos_x = 0;
            self.caret_pos_y += 1;

            if self.caret_pos_y == FB_HEIGHT {
                self.scroll();
                scrolled = true;
                self.caret_pos_y -= 1;
            }
        }

        if scrolled
        { self.flush() }
    }

    fn scroll(&mut self) {
        const LINE_OFF: usize = FB_WIDTH;
        for y in 0..FB_HEIGHT - 1 {
            let off: usize = y * FB_WIDTH;
            for x in 0..FB_WIDTH {
                self.buf[off + x] = self.buf[off + x + LINE_OFF];
            }
        }

        const LAST_LINE_OFF: usize = (FB_HEIGHT - 1) * FB_WIDTH;
        for x in 0..FB_WIDTH {
            self.buf[LAST_LINE_OFF + x] = Cell::new(' ', FbColor::Black, FbColor::White);
        }

        self.dirty_rect.mark_all_screen_dirty();
        self.flush();
    }

    pub fn flush(&mut self)
    {
        for y in self.dirty_rect.start_y..=self.dirty_rect.end_y
        {
            let idx_off = y * FB_WIDTH;
            for x in self.dirty_rect.start_x..=self.dirty_rect.end_x
            {
                let idx = idx_off + x;
                let cell = self.buf[idx];
                unsafe {
                    core::ptr::write_volatile(VGA_MEM.offset((idx * 2) as isize), cell.c as u8);
                    core::ptr::write_volatile(
                        VGA_MEM.offset((idx * 2 + 1) as isize),
                        cell.color_code.into(),
                    );
                }
            }
        }

        self.dirty_rect.reset();
    }
    #[cfg(test)]
    pub fn get_buf(&self) -> &[Cell; FB_WIDTH * FB_HEIGHT] {
        &self.buf
    }
}

impl fmt::Write for Framebuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
