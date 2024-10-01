#[macro_use]
extern crate lazy_static;

extern crate gtk;
use gtk::prelude::*;
use gtk::{FileChooserAction, FileChooserDialog, ResponseType, Window};
use magic::*;
use num_complex::*;
use num_integer::*;
use raylib::ffi::{
    CheckCollisionPointRec, Color, ColorAlpha, DrawCircle, DrawCircleLines, DrawLineEx,
    DrawRectangle, DrawRectangleLines, GetMousePosition, Rectangle, Vector2,
};
use raylib::prelude::*;
use rust_math::trigonometry::deg2rad;
use std::f32::consts::PI;
use std::f32::*;
use std::mem::size_of;

fn array_len<T>(xs: &[T]) -> usize {
    xs.len()
}
const N: usize = 1 << 13;
// Color Palette for Gruvbox
// lazy static is used to initialize the static variables only once at runtime

pub const GRUVBOX_BG: raylib::ffi::Color = raylib::ffi::Color {
    r: 40,
    g: 40,
    b: 40,
    a: 255,
}; // #282828

pub const GRUVBOX_FG: raylib::ffi::Color = raylib::ffi::Color {
    r: 235,
    g: 219,
    b: 178,
    a: 255,
}; // #ebdbb2

pub const GRUVBOX_YELLOW: raylib::ffi::Color = raylib::ffi::Color {
    r: 250,
    g: 189,
    b: 47,
    a: 255,
}; // #fabd2f

pub const GRUVBOX_BLUE: raylib::ffi::Color = raylib::ffi::Color {
    r: 131,
    g: 165,
    b: 152,
    a: 255,
}; // #83a598

pub const GRUVBOX_GREEN: raylib::ffi::Color = raylib::ffi::Color {
    r: 184,
    g: 187,
    b: 38,
    a: 255,
}; // #b8bb26

pub const GRUVBOX_RED: raylib::ffi::Color = raylib::ffi::Color {
    r: 251,
    g: 73,
    b: 52,
    a: 255,
}; // #fb4934

pub const GRUVBOX_ORANGE: raylib::ffi::Color = raylib::ffi::Color {
    r: 254,
    g: 128,
    b: 25,
    a: 255,
}; // #fe8019

pub const GRUVBOX_AQUA: raylib::ffi::Color = raylib::ffi::Color {
    r: 142,
    g: 192,
    b: 124,
    a: 255,
}; // #8ec07c

pub const GRUVBOX_PURPLE: raylib::ffi::Color = raylib::ffi::Color {
    r: 211,
    g: 134,
    b: 155,
    a: 255,
}; // #d3869b

// This derive macro is used to implement the Copy, Clone and Debug traits for the enum
#[derive(Copy, Clone, Debug)]
enum VisualizationMode {
    STANDARD,
    PIXEL,
    WAVEFORM,
    STARBURST,
    RADIAL_BARS,
}

// This implementation of the enum is used to convert the enum to usize and vice versa
impl VisualizationMode {
    fn from_usize(value: usize) -> VisualizationMode {
        match value % NUM_MODES {
            0 => VisualizationMode::STANDARD,
            1 => VisualizationMode::PIXEL,
            2 => VisualizationMode::WAVEFORM,
            3 => VisualizationMode::STARBURST,
            4 => VisualizationMode::RADIAL_BARS,
            _ => VisualizationMode::STANDARD,
        }
    }

    fn as_usize(&self) -> usize {
        match self {
            VisualizationMode::STANDARD => 0,
            VisualizationMode::PIXEL => 1,
            VisualizationMode::WAVEFORM => 2,
            VisualizationMode::STARBURST => 3,
            VisualizationMode::RADIAL_BARS => 4,
        }
    }
}

struct MusicMetadata {
    title: String,
    artist: String,
    album: String,
    duration: f32,
}

// Global Variables
static mut freqs: [f32; N] = [0.0; N];
static mut global_frames: [f32; 4800] = [0.0; 4800];
static mut global_frames_count: usize = 0;
static mut input: [f32; N] = [0.0; N];
static mut output: [Complex<f32>; N] = [Complex::new(0.0, 0.0); N];

static mut max_amp: f32 = 0.0;
static mut selected_song: String = String::new();
const NUM_MODES: usize = 5;
static mut currentMode: VisualizationMode = VisualizationMode::STANDARD;

const helpCommands: [&str; 9] = [
    "f            - Play a media file (GTK file dialog will open)\n",
    "<Space>      - Pause music\n",
    "m            - Toggle mute\n",
    "<UP-ARROW>   - Increase volume by 10%\n",
    "<DOWN-ARROW> - Decrease volume by 10%\n\n",
    "----------------- VISUAL MODES ---------------------\n\n",
    "v            - Cycle through visual modes (forward)\n",
    "b            - Cycle through visual modes (backward)\n",
    "?            - Display the list of available commands",
];

fn fft(inp: &[f32], stride: usize, out: &mut [Complex32], n: usize) {
    assert!(n > 0);

    if n == 1 {
        out[0] = Complex32::new(inp[0], 0.0);
        return;
    }

    fft(&inp, stride * 2, &mut out[..n / 2], n / 2);
    fft(&inp[stride..], stride * 2, &mut out[n / 2..], n / 2);

    for k in 0..n / 2 {
        let t = k as f32 / n as f32;
        let v = Complex32::new(0.0, -2.0 * PI * t).exp() * out[k + n / 2];
        let e = out[k];
        out[k] = e + v;
        out[k + n / 2] = e - v;
    }
}

fn amp(z: Complex32) -> f32 {
    let a = (z.re).abs();
    let b = (z.im);
    if a < b {
        a
    } else {
        b
    }
}

fn SwitchVizualizationModeForward() {
    unsafe {
        let next_mode = (currentMode.as_usize() + 1) % NUM_MODES;
        currentMode = VisualizationMode::from_usize(next_mode);
    }
}

fn SwitchVizualizationModeBackward() {
    unsafe {
        let next_mode = (currentMode.as_usize() - 1) % NUM_MODES;
        currentMode = VisualizationMode::from_usize(next_mode);
    }
}

fn callback(bufferData: *mut [[f32; 2]], frames: usize) {
    unsafe {
        let fs: &mut [[f32; 2]] = &mut *bufferData;

        for i in 0..frames {
            input.copy_within(1..N, 0);
            input[N - 1] = (fs[i][0] + fs[i][1]) / 2.0;
        }

        fft(&input, 1, &mut output, N);

        max_amp = 0.0;
        for i in 0..frames {
            let a: f32 = amp(output[i]);
            if a > max_amp {
                max_amp = a;
            }
        }
    }
}

fn is_song_file(filename: &str) -> bool {
    let extensions = [".mp3", ".wav", ".ogg", ".flac", ".aac"];
    for ext in extensions.iter() {
        if filename.ends_with(ext) {
            return true;
        }
    }
    return false;
}

fn DrawCoolRectangle(x: f32, y: f32, width: f32, height: f32, color: raylib::ffi::Color) {
    unsafe {
        DrawRectangle(x as i32, y as i32, width as i32, height as i32, color);
        DrawRectangleLines(
            x as i32,
            y as i32,
            width as i32,
            height as i32,
            ColorAlpha(color, 0.3),
        );
        DrawCircle(
            (x + width / 2.0) as i32,
            y as i32,
            width / 4.0,
            ColorAlpha(color, 0.2),
        );
    }
}

fn isMouseOverRectangle(rect: Rectangle) -> bool {
    unsafe {
        let mouse = GetMousePosition();
        return CheckCollisionPointRec(mouse, rect);
    }
}

fn OpenFileDialog() -> Option<String> {
    if !gtk::is_initialized() {
        if gtk::init().is_err() {
            eprintln!("Failed to initialize GTK.");
            return None;
        }
    }

    // Create a file chooser dialog with Open and Cancel buttons
    let dialog =
        FileChooserDialog::new(Some("Open File"), None::<&Window>, FileChooserAction::Open);
    dialog.add_buttons(&[
        ("_Cancel", ResponseType::Cancel),
        ("_Open", ResponseType::Accept),
    ]);

    let file_name = if dialog.run() == ResponseType::Accept {
        dialog
            .filename()
            .map(|path| path.to_string_lossy().into_owned())
    } else {
        None
    };

    dialog.close();

    while gtk::events_pending() {
        gtk::main_iteration();
    }

    file_name
}

fn handleVisualization(cell_width: f32, screenHeight: i32, screenWidth: i32, m: usize) {
    unsafe {
        let center: Vector2 = Vector2 {
            x: (screenWidth / 2) as f32,
            y: (screenHeight / 2) as f32,
        }; // Calculating the
           // center point for drawing
        let mut step = 0.4;
        let mut maxAmplitude: f32 = 0.0;
        if max_amp > 0.0 {
            maxAmplitude = max_amp;
        } else {
            maxAmplitude = 1.0;
        }

        // Calculate amplitude for all points at once
        let mut amplitudes: [f32; N] = [0.0; N];
        for i in 0..N {
            amplitudes[i] = amp(output[i]) / maxAmplitude; // Normalize amplitude
        }

        // For storing previous amplitudes for smoothign
        static mut previousAmplitudes: [f32; N] = [0.0; N];

        for i in 0..N - 1 {
            if amplitudes[i] > 0.01 {
                match currentMode {
                    VisualizationMode::STANDARD => DrawCoolRectangle(
                        (i as f32) * cell_width,
                        (screenHeight as f32) - (screenHeight as f32) * amplitudes[i],
                        cell_width * step,
                        (screenHeight as f32) * amplitudes[i],
                        GRUVBOX_RED,
                    ),

                    VisualizationMode::PIXEL => {
                        step = 1.06;
                        DrawCoolRectangle(
                            (i as f32) * cell_width,
                            (screenHeight as f32) - (screenHeight as f32) * amplitudes[i],
                            cell_width * step,
                            (screenHeight as f32) * amplitudes[i],
                            GRUVBOX_PURPLE,
                        );
                    }

                    VisualizationMode::WAVEFORM => {
                        let start: Vector2 = Vector2 {
                            x: (i as f32) * cell_width,
                            y: center.y + ((screenHeight / 2) as f32 * amplitudes[i]),
                        };
                        let end: Vector2 = Vector2 {
                            x: ((i + 1) as f32) * cell_width,
                            y: center.y + ((screenHeight / 2) as f32) * amplitudes[i + 1],
                        };
                        DrawLineEx(start, end, 2.0, GRUVBOX_BLUE);
                    }

                    VisualizationMode::STARBURST => {
                        let angle: f32 = i as f32 * 360.0 / m as f32; // Calculate angle for each ray wrt freuency
                                                                      // range
                        let end: Vector2 = Vector2 {
                            x: center.x
                                + deg2rad(angle).cos()
                                    * amplitudes[i]
                                    * ((screenHeight / 2) as f32),
                            y: center.y
                                + deg2rad(angle).sin()
                                    * amplitudes[i]
                                    * ((screenHeight / 2) as f32),
                        };

                        // Selecting a color based on the index
                        let mut rayColor: raylib::ffi::Color = GRUVBOX_YELLOW;
                        match i % 6 {
                            0 => rayColor = GRUVBOX_YELLOW,
                            1 => rayColor = GRUVBOX_BLUE,
                            2 => rayColor = GRUVBOX_GREEN,
                            3 => rayColor = GRUVBOX_RED,
                            4 => rayColor = GRUVBOX_ORANGE,
                            5 => rayColor = GRUVBOX_PURPLE,
                            _ => rayColor = GRUVBOX_YELLOW,
                        }

                        DrawLineEx(center, end, 2.0, rayColor);
                    }

                    VisualizationMode::RADIAL_BARS => {
                        let angle = i as f32 * 360.0 / m as f32; // Calculate angle for each bar wrt audio
                                                                 // frequency range
                        let innerRadius = screenHeight / 8; // Radius for inner circle
                        let outerRadius = screenHeight / 4; // Base radius for bars
                        let amplitudeScale = screenHeight / 4; // Scaling factor for amplitude

                        // Draw the inner circle
                        DrawCircle(
                            center.x as i32,
                            center.y as i32,
                            innerRadius as f32,
                            GRUVBOX_FG,
                        );
                        DrawCircleLines(
                            center.x as i32,
                            center.y as i32,
                            innerRadius as f32,
                            GRUVBOX_FG,
                        );
                        let start: Vector2 = Vector2 {
                            x: center.x + deg2rad(angle).cos() * (outerRadius as f32),
                            y: center.y + deg2rad(angle).sin() * (outerRadius as f32),
                        };

                        // Use a smoothed amplitude value - by taking the average of previous and current amplitudes
                        let smoothedAmplitude = (previousAmplitudes[i] + amplitudes[i]) * 0.5; // Simple
                                                                                               // averaging
                        previousAmplitudes[i] = smoothedAmplitude; // Store for next frame

                        let end: Vector2 = Vector2 {
                            x: center.x
                                + deg2rad(angle).cos()
                                    * ((smoothedAmplitude
                                        * outerRadius as f32
                                        * amplitudeScale as f32)
                                        as f32),
                            y: center.y
                                + deg2rad(angle).sin()
                                    * ((smoothedAmplitude
                                        * outerRadius as f32
                                        * amplitudeScale as f32)
                                        as f32),
                        };

                        let mut barColor: raylib::ffi::Color = GRUVBOX_YELLOW;
                        match i % 6 {
                            0 => barColor = GRUVBOX_YELLOW,
                            1 => barColor = GRUVBOX_BLUE,
                            2 => barColor = GRUVBOX_GREEN,
                            3 => barColor = GRUVBOX_RED,
                            4 => barColor = GRUVBOX_ORANGE,
                            5 => barColor = GRUVBOX_PURPLE,
                            _ => barColor = GRUVBOX_YELLOW,
                        }
                        DrawLineEx(start, end, cell_width * step, barColor); // Draw the radial bar
                    }
                }
            }
        }
    }
}

fn main() {
    let (mut rl, thread) = raylib::init().size(640, 480).title("Hello, World").build();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(raylib::prelude::Color::WHITE);
        d.draw_text("Hello, world!", 12, 12, 20, raylib::prelude::Color::BLACK);
    }
}
