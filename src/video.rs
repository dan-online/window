use anyhow::Context;
use colored::Colorize;
use image::{ImageBuffer, Rgb};
use ndarray::{ArrayBase, Dim, OwnedRepr};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use termion::{cursor, terminal_size};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::time::Instant;
use video_rs::{DecoderBuilder, Location, Options, Resize, Url};

use crate::utils::args::HardwareAcceleration;
use crate::utils::ffprobe::{
    ffmpeg_initialize, ffprobe_get_duration, ffprobe_get_fps, DurationType,
};
use crate::utils::get_grey::get_grey;
use crate::utils::rgb_distance::rgb_distance;
use crate::utils::size::size;
use crate::utils::youtube::get_youtube_video_from_url;
use crate::{CharacterMode, ScaleMode};

pub type Frame = ArrayBase<OwnedRepr<u8>, Dim<[usize; 3]>>;

pub struct Video {
    pub url: String,
    pub frame_times: Vec<Instant>,
    pub last_frame: Option<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    pub character_mode: CharacterMode,
    pub clear_distance: u16,
    pub scale_mode: ScaleMode,
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

        let ((width, height), _) = size();

        let mut opts: HashMap<String, String> = HashMap::new();

        opts.insert("loglevel".to_string(), "quiet".to_string());
        opts.insert("nostats".to_string(), "1".to_string());

        let options: Options = Options::from(opts);

        let duration = ffprobe_get_duration(&video_url.to_string()).await?;

        let mut decoder = DecoderBuilder::new(video_url)
            .with_resize(match self.scale_mode {
                ScaleMode::Fit => Resize::Fit(width as u32, height as u32 * 2 - 8),
                ScaleMode::Stretch => Resize::Exact(width as u32, height as u32 * 2 - 8),
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

    pub fn print_frame_to_terminal(&mut self, frame: &Frame, stdout: &mut io::Stdout) {
        let frame_height = frame.shape()[0];
        let frame_width = frame.shape()[1];

        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_vec(
            frame_width as u32,
            frame_height as u32,
            frame.as_slice().unwrap().to_vec(),
        )
        .unwrap();

        let mut output = String::new();
        let step_size: u32 = 2; // Adjusted step_size for better visual consistency

        // if frame width is smaller than terminal width, center the frame
        let (terminal_width, _) = terminal_size().unwrap();

        let x_offset: u32 = if frame_width < terminal_width as usize {
            (terminal_width as u32 - frame_width as u32) / 2
        } else {
            0
        };

        let y_offset: u32 = 2;

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
                    rgb_distance((r, g, b), (last_r, last_g, last_b)) >= self.clear_distance as f32
                } else {
                    true
                };

                if needs_update {
                    let grey = get_grey(r, g, b);
                    let ascii = match self.character_mode {
                        CharacterMode::Blocks => "█".truecolor(r, g, b).to_string(),
                        CharacterMode::Dots => "•".on_black().truecolor(r, g, b).to_string(),
                        CharacterMode::Ascii => {
                            let c = match grey {
                                0..=42 => "@",
                                43..=85 => "#",
                                86..=127 => "+",
                                128..=170 => ":",
                                171..=212 => ".",
                                213..=255 => "-",
                            };
                            c.truecolor(128, 128, 128).on_truecolor(r, g, b).to_string()
                        }
                        CharacterMode::Numbers => {
                            let c = match grey {
                                0..=42 => "8",
                                43..=85 => "5",
                                86..=127 => "3",
                                128..=170 => "2",
                                171..=212 => "1",
                                213..=255 => "0",
                            };
                            c.truecolor(128, 128, 128).on_truecolor(r, g, b).to_string()
                        }
                        CharacterMode::Unicode => {
                            let c = match grey {
                                0..=63 => "█",
                                64..=127 => "▓",
                                128..=191 => "▒",
                                192..=255 => "░",
                            };
                            c.on_black().truecolor(r, g, b).to_string()
                        }
                    };

                    output.push_str(&format!(
                        "{}{}",
                        cursor::Goto(
                            (x + x_offset) as u16,
                            ((y / step_size + 1) + y_offset) as u16
                        ),
                        ascii
                    ));
                }
            }
        }

        write!(stdout, "{}", output).unwrap();
        self.last_frame = Some(img);
        self.frame_times.push(Instant::now());
        io::stdout().flush().unwrap();
    }

    pub fn fps(&mut self) -> f64 {
        let mut frame_count = self.frame_times.len();

        if frame_count > 10 {
            self.frame_times = self.frame_times[frame_count - 10..].to_vec();

            frame_count = self.frame_times.len();
        }

        if frame_count < 5 {
            return 0.0;
        }

        let start = self.frame_times[0];
        let end = self.frame_times[frame_count - 1];

        let elapsed = end - start;

        frame_count as f64 / elapsed.as_secs_f64()
    }
}
