use clap::Parser;
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
use utils::{
    args::{Args, CharacterMode, ScaleMode},
    ffprobe::DurationType,
    format_time::format_time,
};
use video::{Frame, Video};

mod video;
mod utils {
    pub mod args;
    pub mod ffprobe;
    pub mod format_time;
    pub mod get_grey;
    pub mod rgb_distance;
    pub mod size;
    pub mod youtube;
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
    let (render_tx, render_recv) = unbounded_channel::<(Frame, DurationType)>();

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
    mut render_recv: UnboundedReceiver<(Frame, DurationType)>,
) {
    let started = Instant::now();
    let std_frame_time = Duration::from_micros(1_000_000 / fps);
    let mut frames_seen = 0;
    let mut last_frame = Instant::now();
    let mut last_frame_speed = Duration::from_micros(0);

    while let Some((frame, duration)) = render_recv.recv().await {
        let elapsed = last_frame.elapsed();

        if frame_cap && elapsed < std_frame_time && last_frame_speed <= (std_frame_time - elapsed) {
            tokio::time::sleep(std_frame_time - elapsed - last_frame_speed).await;
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

        last_frame_speed = elapsed;

        write!(stdout, "{}{}", cursor::Goto(1, height), clear::CurrentLine).unwrap();

        let current_time = frames_seen as f32 / fps as f32;

        let frame_time = format!(
            "{:>width$}ms",
            format!("{:.2}", elapsed.as_secs_f64() * 1000.0),
            // over 1000ms is unlikely, and if so then they have other problems
            width = 6
        );

        let fps_text = format!("FPS: {:.0}/{}", video.fps(), fps);

        let (current_time_str, duration_str, progress_bar) = match duration {
            DurationType::Fixed(duration) => {
                let duration = duration as f32;
                let progress = current_time / duration;

                let current_time_str = format_time(current_time as u64);
                let duration_str = format_time(duration as u64);

                let space = wid as usize
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

                let space = wid as usize
                    - current_time_str.len()
                    - duration_str.len()
                    - fps_text.len()
                    - frame_time.len()
                    - 16;

                let watched_space =
                    ((start - started).as_secs_f32() * 10.0 % space as f32) as usize;

                let remaining_space = space - watched_space;

                let progress_bar = format!(
                    "[{}<=====>{}]",
                    " ".repeat(watched_space),
                    " ".repeat(remaining_space)
                );

                (current_time_str, duration_str, progress_bar)
            }
        };

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

        if let DurationType::Fixed(duration) = duration {
            if (duration as f32 - current_time) < 0.05 {
                end();
            }
        }

        last_frame = Instant::now();
    }

    end();
}
