pub const CURSOR_WIDTH: usize = 12;
pub const CURSOR_HEIGHT: usize = 19;

// The "Hotspot" is the exact pixel that does the clicking (tip of the arrow)
pub const HOTSPOT_X: usize = 0;
pub const HOTSPOT_Y: usize = 0;

// Raw sprite data (0=Trans, 1=Black, 2=White)
#[rustfmt::skip] // Keep it readable as a grid
pub const SPRITE: [u8; CURSOR_WIDTH * CURSOR_HEIGHT] = [
    1,1,0,0,0,0,0,0,0,0,0,0,
    1,2,1,0,0,0,0,0,0,0,0,0,
    1,2,2,1,0,0,0,0,0,0,0,0,
    1,2,2,2,1,0,0,0,0,0,0,0,
    1,2,2,2,2,1,0,0,0,0,0,0,
    1,2,2,2,2,2,1,0,0,0,0,0,
    1,2,2,2,2,2,2,1,0,0,0,0,
    1,2,2,2,2,2,2,2,1,0,0,0,
    1,2,2,2,2,2,2,2,2,1,0,0,
    1,2,2,2,2,1,1,1,1,1,1,0,
    1,2,2,1,2,2,1,0,0,0,0,0,
    1,2,1,0,1,2,2,1,0,0,0,0,
    1,1,0,0,1,2,2,1,0,0,0,0,
    0,0,0,0,0,1,2,2,1,0,0,0,
    0,0,0,0,0,1,2,2,1,0,0,0,
    0,0,0,0,0,0,1,2,2,1,0,0,
    0,0,0,0,0,0,1,2,2,1,0,0,
    0,0,0,0,0,0,0,1,1,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,
];