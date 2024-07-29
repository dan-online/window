use clap::Parser;
use crossterm::{
    cursor::{self, MoveTo},
    execute, queue,
    style::{Print, ResetColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use std::{process::exit, time::Duration};
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
    pub mod youtube;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize video with parameters
    let video = Video {
        url: args.url,
        frame_times: vec![],
        last_frame: None,
        fullscreen: args.fullscreen,
        cap_framerate: args.cap_framerate.unwrap_or(true),
        character_mode: args.mode.unwrap_or(CharacterMode::Blocks),
        pixel_clear_distance: args.pixel_clear_distance.unwrap_or(2),
        scale_mode: args.scale.unwrap_or(ScaleMode::Fit),
    };

    // Fetch video frames and frames per second
    let (mut frames_recv, title, fps) = video
        .fetch_video(args.hw_accel.unwrap_or_default())
        .await
        .unwrap();
    let (render_tx, render_recv) = unbounded_channel::<(Frame, DurationType)>();

    // Clear the terminal screen and hide the cursor
    // print!("{}", cursor::Hide);
    // print!("{}", clear::All);
    let mut stdout = io::stdout();

    execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
    execute!(stdout, cursor::Hide)?;

    stdout.flush().unwrap();

    // Spawn a task to handle signal input
    tokio::spawn(handle_signal_input());

    // Spawn a task to render video frames
    let handle_render = tokio::spawn(handle_render(video, title, fps, render_recv));

    // Forward frames to the render task
    while let Some(data) = frames_recv.recv().await {
        render_tx.send(data).unwrap();
    }

    handle_render.await?
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

fn calculate_fps(frame_times: &Vec<Instant>) -> f64 {
    let frame_count = frame_times.len();

    if frame_count < 10 {
        return 0.0;
    }

    let start = frame_times[0];
    let end = frame_times[frame_count - 1];
    let elapsed = end.duration_since(start);

    if elapsed.as_secs_f64() == 0.0 {
        return 0.0;
    }

    frame_count as f64 / elapsed.as_secs_f64()
}

// Render video frames to the terminal
async fn handle_render(
    mut video: Video,
    title: String,
    fps: u64,
    mut render_recv: UnboundedReceiver<(Frame, DurationType)>,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let std_frame_time = Duration::from_micros(1_000_000 / fps);
    let mut frames_seen = 0;
    let mut frame_times: Vec<Instant> = vec![];

    let mut stdout = io::stdout();
    let (cols, rows) = terminal::size()?;

    if !video.fullscreen {
        let playing_text = format!(" Playing: {} ", title);
        let resolution_text = format!("Resolution: {}x{}", cols, rows);

        queue!(
            stdout,
            MoveTo(0, 0),
            ResetColor,
            Print(format!(
                "{}{}{}",
                playing_text,
                " ".repeat(cols as usize - playing_text.len() - resolution_text.len()),
                resolution_text
            ))
        )
        .unwrap();
    }

    while let Some((frame, duration)) = render_recv.recv().await {
        frames_seen += 1;

        let (wid, height) = terminal::size().unwrap();

        let start = Instant::now();

        video.print_frame_to_terminal(&frame, &mut stdout)?;

        let elapsed = start.elapsed();
        let sleep_duration = std_frame_time.saturating_sub(elapsed);

        // Wait if necessary to maintain the target FPS
        if video.cap_framerate {
            tokio::time::sleep(sleep_duration).await;
        }

        stdout.flush().unwrap();

        frame_times.push(Instant::now());

        let render_fps = calculate_fps(&frame_times);

        if frame_times.len() > 10 {
            frame_times = frame_times[frame_times.len() - 10..].to_vec();
        }

        let current_time = frames_seen as f32 / fps as f32;

        if !video.fullscreen {
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

                    let bar = "<=====>";

                    let space = wid as usize
                        - current_time_str.len()
                        - duration_str.len()
                        - fps_text.len()
                        - frame_time.len()
                        - bar.len()
                        - 9;

                    let watched_space =
                        ((start - started).as_secs_f32() * 10.0 % space as f32) as usize;

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
            .unwrap();
        }

        stdout.flush().unwrap();

        if let DurationType::Fixed(duration) = duration {
            if (duration as f32 - current_time) < 0.05 {
                end();
            }
        }
    }

    end();

    Ok(())
}
