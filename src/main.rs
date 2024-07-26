use clap::Parser;
use serde::Serialize;
use std::{
    io::{self, Write},
    process::exit,
    time::Duration,
};
use termion::{clear, cursor, terminal_size};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
    time::Instant,
};
use utils::format_time::format_time;
use video::{Frame, Video};
use video_rs::hwaccel::HardwareAccelerationDeviceType;

mod video;
mod utils {
    pub mod ffprobe;
    pub mod format_time;
    pub mod get_grey;
    pub mod rgb_distance;
    pub mod size;
    pub mod youtube;
}

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CharacterMode {
    #[default]
    Blocks,
    Dots,
    Ascii,
    Numbers,
    Unicode,
}

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ScaleMode {
    #[default]
    Fit,
    Stretch,
}

// Hardware acceleration device type but clap compatible
#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum HardwareAcceleration {
    #[default]
    None,
    /// Video Decode and Presentation API for Unix (VDPAU)
    Vdpau,
    /// NVIDIA CUDA
    Cuda,
    /// Video Acceleration API (VA-API)
    VaApi,
    /// DirectX Video Acceleration 2.0
    Dxva2,
    /// Quick Sync Video
    Qsv,
    /// VideoToolbox
    VideoToolbox,
    /// Direct3D 11 Video Acceleration
    D3D11Va,
    /// Linux Direct Rendering Manager
    Drm,
    /// OpenCL
    OpenCl,
    /// MediaCodec
    MeiaCodec,
    /// Vulkan
    Vulkan,
    /// Direct3D 12 Video Acceleration
    D3D12Va,
}

impl HardwareAcceleration {
    fn to_video_rs(&self) -> Option<HardwareAccelerationDeviceType> {
        match self {
            HardwareAcceleration::None => None,
            HardwareAcceleration::Vdpau => Some(HardwareAccelerationDeviceType::Vdpau),
            HardwareAcceleration::Cuda => Some(HardwareAccelerationDeviceType::Cuda),
            HardwareAcceleration::VaApi => Some(HardwareAccelerationDeviceType::VaApi),
            HardwareAcceleration::Dxva2 => Some(HardwareAccelerationDeviceType::Dxva2),
            HardwareAcceleration::Qsv => Some(HardwareAccelerationDeviceType::Qsv),
            HardwareAcceleration::VideoToolbox => {
                Some(HardwareAccelerationDeviceType::VideoToolbox)
            }
            HardwareAcceleration::D3D11Va => Some(HardwareAccelerationDeviceType::D3D11Va),
            HardwareAcceleration::Drm => Some(HardwareAccelerationDeviceType::Drm),
            HardwareAcceleration::OpenCl => Some(HardwareAccelerationDeviceType::OpenCl),
            HardwareAcceleration::MeiaCodec => Some(HardwareAccelerationDeviceType::MeiaCodec),
            HardwareAcceleration::Vulkan => Some(HardwareAccelerationDeviceType::Vulkan),
            HardwareAcceleration::D3D12Va => Some(HardwareAccelerationDeviceType::D3D12Va),
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    url: String,

    #[clap(short, long, default_value = "2")]
    clear_distance: Option<u16>,

    #[clap(short, long, default_value = "blocks")]
    mode: Option<CharacterMode>,

    #[clap(short, long)]
    scale: Option<ScaleMode>,

    #[clap(short, long, default_value = "true")]
    frame_cap: Option<bool>,

    #[clap(long)]
    hw_accel: Option<HardwareAcceleration>,
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize video with parameters
    let video = Video {
        url: args.url,
        frame_times: vec![],
        last_frame: None,
        character_mode: args.mode.unwrap_or(CharacterMode::Blocks),
        clear_distance: args.clear_distance.unwrap_or(2),
        scale_mode: args.scale.unwrap_or(ScaleMode::Fit),
    };

    // Fetch video frames and frames per second
    let (mut frames_recv, title, fps) = video
        .fetch_video(args.hw_accel.unwrap_or_default())
        .await
        .unwrap();
    let (render_tx, render_recv) = unbounded_channel::<(Frame, u64)>();

    // Clear the terminal screen and hide the cursor
    print!("{}", cursor::Hide);
    print!("{}", clear::All);

    // Spawn a task to handle signal input
    tokio::spawn(handle_signal_input());

    // Spawn a task to render video frames
    let handle_render = tokio::spawn(handle_render(
        video,
        title,
        fps,
        args.frame_cap.unwrap_or(true),
        render_recv,
    ));

    // Forward frames to the render task
    while let Some(data) = frames_recv.recv().await {
        render_tx.send(data).unwrap();
    }

    // Wait for the render task to complete
    handle_render.await.unwrap();
}

fn end() {
    print!("{}", cursor::Show);
    exit(0);
}

// Handle signal to quit the application
async fn handle_signal_input() {
    tokio::signal::ctrl_c().await.unwrap();
    println!("\n Exit signal received, quitting...");
    end();
}

// Render video frames to the terminal
async fn handle_render(
    mut video: Video,
    title: String,
    fps: u64,
    frame_cap: bool,
    mut render_recv: UnboundedReceiver<(Frame, u64)>,
) {
    let mut frames_seen = 0;
    let mut last_frame = Instant::now();

    while let Some((frame, duration)) = render_recv.recv().await {
        let elapsed = last_frame.elapsed();

        if frame_cap && elapsed < Duration::from_micros(1_000_000 / fps) {
            tokio::time::sleep(Duration::from_micros(1_000_000 / fps) - elapsed).await;
        }

        frames_seen += 1;

        let (wid, height) = terminal_size().unwrap();

        let mut stdout = io::stdout();

        // write top line, TODO: make this not need to be rendered every frame
        write!(stdout, "{}{}", cursor::Goto(1, 1), clear::CurrentLine).unwrap();
        write!(stdout, " Playing: {}", title).unwrap();

        let start = Instant::now();

        video.print_frame_to_terminal(&frame, &mut stdout);

        let elapsed = start.elapsed();

        write!(stdout, "{}{}", cursor::Goto(1, height), clear::CurrentLine).unwrap();

        let current_time = frames_seen as f32 / fps as f32;
        let duration = duration as f32;

        let fps_text = format!("FPS: {:.0}/{}", video.fps(), fps);
        let frame_time = format!(
            // pad left with spaces
            "{:>width$}ms",
            format!("{:.2}", elapsed.as_secs_f64() * 1000.0),
            // over 1000ms is unlikely, and if so then they have other problems
            width = 6
        );

        let progress = current_time / duration;

        let current_time_str = format_time(current_time as u64);
        let duration_str = format_time(duration as u64);

        let space = wid as usize
            - current_time_str.len()
            - duration_str.len()
            - fps_text.len()
            - frame_time.len()
            - 10;

        let watched_space = (progress * space as f32) as usize;
        let remaining_space = space - watched_space;

        let progress_bar = format!(
            "[{}{}]",
            "=".repeat(watched_space),
            " ".repeat(remaining_space)
        );

        write!(
            stdout,
            "{}{} {}/{} {} {} {} ",
            cursor::Goto(1, height),
            clear::CurrentLine,
            current_time_str,
            duration_str,
            progress_bar,
            fps_text,
            frame_time
        )
        .unwrap();

        stdout.flush().unwrap();

        if (duration - current_time) < 0.1 {
            end();
        }

        last_frame = Instant::now();
    }

    end();
}
