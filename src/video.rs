use anyhow::Context;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{queue, terminal};
use image::{ImageBuffer, Rgb};
use ndarray::{ArrayBase, Dim, OwnedRepr};
use std::collections::HashMap;
use std::io::{self};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::time::Instant;
use video_rs::{DecoderBuilder, Location, Options, Resize, Url};

use crate::utils::args::HardwareAcceleration;
use crate::utils::ffprobe::{
    ffmpeg_initialize, ffprobe_get_duration, ffprobe_get_fps, DurationType,
};
use crate::utils::format_time::format_time;
use crate::utils::get_grey::get_grey;
use crate::utils::rgb_distance::rgb_distance;
use crate::utils::youtube::get_youtube_video_from_url;
use crate::{CharacterMode, ScaleMode};

pub type Frame = ArrayBase<OwnedRepr<u8>, Dim<[usize; 3]>>;

pub struct Video {
    pub url: String,
    pub frame_times: Vec<Instant>,
    pub last_frame: Option<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    pub character_mode: CharacterMode,
    pub pixel_clear_distance: u16,
    pub scale_mode: ScaleMode,
    pub cap_framerate: bool,
    pub fullscreen: bool,
}

enum VideoUrl {
    YoutubeUrl(String),
    File(String),
    DirectUrl(String),
}

impl std::str::FromStr for VideoUrl {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("http") {
            if s.contains("youtube.com") || s.contains("youtu.be") {
                return Ok(Self::YoutubeUrl(s.to_string()));
            }

            return Ok(Self::DirectUrl(s.to_string()));
        }

        Ok(Self::File(s.to_string()))
    }
}

fn get_step_size() -> u32 {
    let (width, height) = terminal::size().unwrap();

    return (width / (height - 2)) as u32 - 2;
}

impl Video {
    pub async fn fetch_video(
        &self,
        hw_accel: HardwareAcceleration,
    ) -> anyhow::Result<(UnboundedReceiver<(Frame, DurationType)>, String, u64)> {
        ffmpeg_initialize()?;

        let video_type = self.url.parse::<VideoUrl>().unwrap();

        let (video_url, fps, title) = match video_type {
            VideoUrl::YoutubeUrl(url) => {
                let (video_url, fps, title) = get_youtube_video_from_url(&url)
                    .with_context(|| format!("Failed to get video from {}", url))?;

                (
                    Location::Network(video_url.parse::<Url>().unwrap()),
                    fps,
                    title,
                )
            }

            VideoUrl::File(path) => {
                let fps = ffprobe_get_fps(&path)
                    .with_context(|| format!("Failed to get fps for {}", path))?;

                (Location::File(PathBuf::from(path.clone())), fps, path)
            }

            VideoUrl::DirectUrl(url) => {
                let fps = ffprobe_get_fps(&url)
                    .with_context(|| format!("Failed to get fps for {}", url))?;

                (Location::Network(url.parse::<Url>().unwrap()), fps, url)
            }
        };

        let (width, height) = terminal::size().unwrap();

        let mut opts: HashMap<String, String> = HashMap::new();

        opts.insert("loglevel".to_string(), "quiet".to_string());
        opts.insert("nostats".to_string(), "1".to_string());

        let options: Options = Options::from(opts);

        let duration = ffprobe_get_duration(&video_url.to_string()).await?;

        let step_size = get_step_size();

        let mut render_height = height as u32 * step_size;
        let render_width = width as u32;

        if !self.fullscreen {
            render_height -= 8;
        }

        let mut decoder = DecoderBuilder::new(video_url)
            .with_resize(match self.scale_mode {
                ScaleMode::Fit => Resize::Fit(render_width, render_height),
                ScaleMode::Stretch => Resize::Exact(render_width, render_height),
            })
            .with_options(&options);

        if hw_accel != HardwareAcceleration::None {
            decoder = decoder.with_hardware_acceleration(hw_accel.to_video_rs().unwrap());
        }

        let mut decoder = decoder.build().expect("failed to create decoder");

        let (frame_tx, frame_rx) = unbounded_channel();

        tokio::spawn(async move {
            while let Ok((_, frame)) = decoder.decode() {
                frame_tx.send((frame, duration)).unwrap();
            }
        });

        Ok((frame_rx, title, fps))
    }

    pub fn print_frame_to_terminal(
        &mut self,
        frame: &Frame,
        stdout: &mut io::Stdout,
    ) -> anyhow::Result<()> {
        let frame_height = frame.shape()[0];
        let frame_width = frame.shape()[1];

        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_vec(
            frame_width as u32,
            frame_height as u32,
            frame.as_slice().unwrap().to_vec(),
        )
        .unwrap();

        let step_size: u32 = get_step_size();

        let (terminal_width, _) = terminal::size().unwrap();

        let x_offset: u32 = if frame_width < terminal_width as usize {
            (terminal_width as u32 - frame_width as u32) / 2
        } else {
            0
        };

        let y_offset: u32 = if !self.fullscreen { 2 } else { 0 };

        let background = match self.character_mode {
            CharacterMode::Blocks | CharacterMode::Dots => None,
            _ => None,
        };

        if let Some(background) = background {
            queue!(stdout, SetBackgroundColor(background))?;
        }

        let mut last_bg: Option<Color> = None;
        let mut last_fg: Option<Color> = None;

        for y in (0..img.height()).step_by(step_size as usize) {
            for x in 0..img.width() {
                let pixel = img.get_pixel(x, y);
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];

                let needs_update = if let Some(last_frame) = &self.last_frame {
                    let last_pixel = last_frame.get_pixel(x, y);
                    let last_r = last_pixel[0];
                    let last_g = last_pixel[1];
                    let last_b = last_pixel[2];
                    rgb_distance((r, g, b), (last_r, last_g, last_b))
                        >= self.pixel_clear_distance as f32
                } else {
                    true
                };

                if needs_update {
                    let grey = get_grey(r, g, b);

                    let ramp: Vec<u32> = match self.character_mode {
                        // CharacterMode::Blocks => b"█",
                        CharacterMode::Blocks => [0x2588].to_vec(),
                        // CharacterMode::Dots => b"•",
                        CharacterMode::Dots => [0x2022].to_vec(),
                        // CharacterMode::Dots => [0x25A0].to_vec(),
                        CharacterMode::Ascii => {
                            b"@#%*+=-:. ".to_vec().iter().map(|&x| x as u32).collect()
                        }
                        // CharacterMode::AsciiExtended =>
                        //     b"$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\" ^ `'. ".to_vec().iter().map(|&x| x as u32).collect(),
                        CharacterMode::AsciiExtended => {
                            " .'`^\",:;Il!i><~+_-?][}{1)(|\\//tfjrxnuvczXUYJCLQ0OZmwqpbdkhao*#M&W&8%B@$".to_string().chars().map(|x| x as u32).collect()
                        }
                        CharacterMode::Numbers => {
                            b"853210".to_vec().iter().map(|&x| x as u32).collect()
                        }
                        // CharacterMode::Unicode => b"▓▒▒▓",
                        // CharacterMode::Unicode => [0x2588, 0x2593, 0x2592, 0x2591].to_vec(),
                        // 0x2591 == ░
                        // 0x2592 == ▒
                        // 0x2593 == ▓
                        CharacterMode::Unicode => [0x2591, 0x2592].to_vec(),
                        // [0x2588, 0x259A,0x25A0
                    };

                    let ramp_len = ramp.len() as f32;
                    let ramp_index = (grey as f32 / 255.0 * (ramp_len - 1.0)).round() as usize;

                    let ascii = char::from_u32(ramp[ramp_index]).unwrap();

                    let color = match self.character_mode {
                        CharacterMode::Blocks | CharacterMode::Dots => Color::Rgb { r, g, b },
                        CharacterMode::Ascii
                        | CharacterMode::Numbers
                        | CharacterMode::Unicode
                        | CharacterMode::AsciiExtended => Color::Rgb {
                            r: 128,
                            g: 128,
                            b: 128,
                        },
                    };

                    if background.is_none() && last_bg != Some(Color::Rgb { r, g, b }) {
                        queue!(stdout, SetBackgroundColor(Color::Rgb { r, g, b }))?;
                    }

                    queue!(
                        stdout,
                        MoveTo((x + x_offset) as u16, ((y / step_size) + y_offset) as u16),
                    )?;

                    if last_fg != Some(color) {
                        queue!(stdout, SetForegroundColor(color))?;
                    }

                    queue!(stdout, Print(ascii))?;

                    last_bg = Some(Color::Rgb { r, g, b });
                    last_fg = Some(color);
                }
            }
        }

        self.last_frame = Some(img);
        self.frame_times.push(Instant::now());

        Ok(())
    }

    pub fn print_footer_to_terminal(
        &self,
        stdout: &mut io::Stdout,
        fps: u64,
        render_fps: f64,
        current_time: f32,
        duration: DurationType,
        elapsed: Duration,
        time_since_start: Duration,
    ) -> anyhow::Result<()> {
        let (width, height) = terminal::size().unwrap();

        queue!(
            stdout,
            MoveTo(0, height),
            ResetColor,
            Clear(ClearType::CurrentLine)
        )
        .unwrap();

        let frame_time = format!(
            "{:>width$}ms",
            format!("{:.2}", elapsed.as_secs_f64() * 1000.0),
            // over 1000ms is unlikely, and if so then they have other problems
            width = 6
        );

        let fps_text = format!("FPS: {:.0}/{}", render_fps, fps);

        let (current_time_str, duration_str, progress_bar) = match duration {
            DurationType::Fixed(duration) => {
                let duration = duration as f32;
                let progress = current_time / duration;

                let current_time_str = format_time(current_time as u64);
                let duration_str = format_time(duration as u64);

                let space = width as usize
                    - current_time_str.len()
                    - duration_str.len()
                    - fps_text.len()
                    - frame_time.len()
                    - 10;

                let watched_space = (progress * (space as f32)) as usize;
                let remaining_space = (space - watched_space) as usize;

                let progress_bar = format!(
                    "[{}{}]",
                    "=".repeat(watched_space),
                    " ".repeat(remaining_space)
                );

                (current_time_str, duration_str, progress_bar)
            }
            DurationType::Live => {
                let frame_time = format!(
                    // pad left with spaces
                    "{:>width$}ms",
                    format!("{:.2}", elapsed.as_secs_f64() * 1000.0),
                    // over 1000ms is unlikely, and if so then they have other problems
                    width = 6
                );

                let current_time_str = format_time(current_time as u64);

                let duration_str = "Live".to_string();

                let bar = "<=====>";

                let space = width as usize
                    - current_time_str.len()
                    - duration_str.len()
                    - fps_text.len()
                    - frame_time.len()
                    - bar.len()
                    - 9;

                let watched_space = (time_since_start.as_secs_f32() * 10.0 % space as f32) as usize;

                let remaining_space = space.saturating_sub(watched_space);

                let progress_bar = format!(
                    "[{}{}{}]",
                    " ".repeat(watched_space),
                    bar,
                    " ".repeat(remaining_space)
                );

                (current_time_str, duration_str, progress_bar)
            }
        };

        queue!(
            stdout,
            Print(format!(
                " {}/{} {} {} {} ",
                current_time_str, duration_str, progress_bar, fps_text, frame_time
            ))
        )
        .map_err(|e| anyhow::anyhow!(e))
    }
}
