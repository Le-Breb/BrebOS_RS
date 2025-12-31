use crate::framebuffer::Color::{Black, White};
use core::fmt;
use spin::Mutex;
use lazy_static::lazy_static;

const FB_WIDTH: usize = 80;
const FB_HEIGHT: usize = 25;
const VGA_MEM: *mut u8 = 0xB8000 as *mut u8;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        write!(framebuffer::FB.lock(), $($arg)*).unwrap()
    };
}

#[macro_export]
macro_rules! println {
    () => {
        write!(framebuffer::FB.lock(), "\n").unwrap()
    };
    ($($arg:tt)*) => {
        write!(framebuffer::FB.lock(), "{}\n", format_args!($($arg)*)).unwrap()
    };
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
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
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

lazy_static! {
    pub static ref FB: Mutex<Framebuffer> = Mutex::new(Framebuffer::new());
}

#[derive(Copy, Clone)]
struct Cell {
    c: char,
    color_code: ColorCode,
}

impl From<ColorCode> for u8 {
    fn from(c: ColorCode) -> Self {
        c.0
    }
}

impl Cell {
    pub fn new(c: char, bg: Color, fg: Color) -> Self {
        Cell {
            c,
            color_code: ColorCode::new(fg, bg),
        }
    }
}

pub struct Framebuffer {
    caret_pos_x: usize,
    caret_pos_y: usize,
    buf: [Cell; FB_WIDTH * FB_HEIGHT],
}

impl Framebuffer {
    pub fn new() -> Self {
        Framebuffer {
            caret_pos_x: 0,
            caret_pos_y: 0,
            buf: [Cell::new(' ', Color::Black, Color::White); FB_WIDTH * FB_HEIGHT],
        }
    }

    pub fn write_str(&mut self, string: &str) {
        for c in string.chars() {
            self.write_char(c);
        }
    }

    pub fn write_char(&mut self, c: char) {
        self.write_cell(Cell::new(c, Black, White))
    }

    fn write_cell(&mut self, cell: Cell) {
        let pos: usize = self.caret_pos_y * FB_WIDTH + self.caret_pos_x;
        unsafe {
            core::ptr::write_volatile(VGA_MEM.offset((pos * 2) as isize), cell.c as u8);
            core::ptr::write_volatile(
                VGA_MEM.offset((pos * 2 + 1) as isize),
                cell.color_code.into(),
            );
        }
        self.buf[pos] = cell;

        self.caret_pos_x += 1;
        if self.caret_pos_x == FB_WIDTH {
            self.caret_pos_x = 0;
            self.caret_pos_y += 1;

            if self.caret_pos_y == FB_HEIGHT {
                self.scroll();
                self.caret_pos_y -= 1;
            }
        }
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
            self.buf[LAST_LINE_OFF + x] = Cell::new(' ', Black, White);
        }

        for (idx, cell) in self.buf.iter().enumerate() {
            unsafe {
                core::ptr::write_volatile(VGA_MEM.offset((idx * 2) as isize), cell.c as u8);
                core::ptr::write_volatile(
                    VGA_MEM.offset((idx * 2 + 1) as isize),
                    cell.color_code.into(),
                );
            }
        }
    }
}

impl fmt::Write for Framebuffer
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}