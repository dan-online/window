use clap::Parser;
use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
use crossterm::{
    cursor::{self, MoveTo},
    execute,
    style::{self},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use std::sync::Arc;
use std::{process::exit, time::Duration};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
    time::Instant,
};
use utils::{
    args::{Args, CharacterMode, ScaleMode},
    calculate_fps::calculate_fps,
    ffprobe::DurationType,
};
use video::{Frame, Video};

mod video;
mod utils {
    pub mod args;
    pub mod calculate_fps;
    pub mod ffprobe;
    pub mod format_time;
    pub mod get_grey;
    pub mod rgb_distance;
    pub mod step_size;
    pub mod youtube;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize "video" with parameters
    let mut video = Video::from_args(args);

    // Fetch video frames and frames per second
    let (mut frames_recv, seek_tx) = video.fetch_video(video.hw_accel.clone()).await.unwrap();
    let (render_tx, render_recv) = unbounded_channel::<(Frame, DurationType)>();

    let mut stdout = io::stdout();

    execute!(stdout, Clear(ClearType::All))?;
    execute!(stdout, cursor::Hide)?;

    // Spawn a task to handle signal input
    tokio::spawn(handle_signal_input());

    // Spawn a task to render video frames
    let handle_render = tokio::spawn(handle_render(video, seek_tx, render_recv));

    // Forward frames to the render task
    while let Some(data) = frames_recv.recv().await {
        render_tx.send(data).unwrap();
    }

    let _ = handle_render.await?;

    Ok(())
}

fn end() {
    terminal::disable_raw_mode().unwrap();
    print!(
        "{}{}{}{}",
        cursor::Show,
        style::ResetColor,
        MoveTo(0, 0),
        Clear(ClearType::All)
    );
    exit(0);
}

// Handle signal to quit the application
async fn handle_signal_input() {
    tokio::signal::ctrl_c().await.unwrap();
    end();
}

// Render video frames to the terminal
async fn handle_render(
    mut video: Video,
    seek_tx: UnboundedSender<i64>,
    mut render_recv: UnboundedReceiver<(Frame, DurationType)>,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let std_frame_time = Duration::from_micros(1_000_000 / video.fps);
    let frames_seen = Arc::new(RwLock::new(0));
    let mut frame_times: Vec<Instant> = vec![];

    let mut stdout = io::stdout();

    let (mut last_width, mut last_height) = terminal::size()?;

    terminal::enable_raw_mode()?;

    let frames_seen_copy = frames_seen.clone();

    tokio::spawn(async move {
        loop {
            let ev = read().unwrap();
            match ev {
                Event::Key(event) => {
                    if event.code == KeyCode::Char('q')
                        || (event.code == KeyCode::Char('c')
                            && event.modifiers == KeyModifiers::CONTROL)
                    {
                        end();
                    }

                    if !video.live {
                        if event.code == KeyCode::Char('l') {
                            let mut frames_seen = frames_seen_copy.write().await;
                            let current_time = *frames_seen as f32 / video.fps as f32;

                            seek_tx
                                .send((current_time * 1000.0 + 5000.0) as i64)
                                .unwrap();

                            // set frames_seen to new timestamp
                            *frames_seen = ((current_time + 5.0) * (video.fps as f32)) as u64;
                            drop(frames_seen);
                        }

                        if event.code == KeyCode::Char('k') {
                            let mut frames_seen = frames_seen_copy.write().await;
                            let current_time = *frames_seen as f32 / video.fps as f32;

                            seek_tx
                                .send((current_time * 1000.0 - 5000.0) as i64)
                                .unwrap();

                            *frames_seen =
                                ((current_time - 5.0).max(0.0) * (video.fps as f32)) as u64;
                            drop(frames_seen);
                        }
                    }

                    // if event.code == KeyCode::Char('f') {
                    //     video.fullscreen = !video.fullscreen;
                    // } Doesn't work, prolly due to the spawn
                }
                _ => {}
            }
        }
    });

    while let Some((frame, duration)) = render_recv.recv().await {
        let (width, height) = terminal::size()?;

        if width != last_width || height != last_height {
            execute!(stdout, Clear(ClearType::All))?;
            last_width = width;
            last_height = height;
        }

        let mut frames_seen_write_lock = frames_seen.write().await;

        *frames_seen_write_lock += 1;

        drop(frames_seen_write_lock);

        video.write_header(&mut stdout)?;

        let start = Instant::now();

        video.write_frame(&frame, &mut stdout)?;

        let elapsed = start.elapsed();
        let sleep_duration = std_frame_time.saturating_sub(elapsed);

        // Wait if necessary to maintain the target FPS with a preloaded video
        if !video.remove_fps_cap {
            tokio::time::sleep(sleep_duration).await;
        }

        stdout.flush().unwrap();

        frame_times.push(Instant::now());

        let render_fps = calculate_fps(&frame_times);

        if frame_times.len() > 10 {
            frame_times = frame_times[frame_times.len() - 10..].to_vec();
        }

        let frames_seen = frames_seen.read().await;

        let current_time = *frames_seen as f32 / video.fps as f32;

        drop(frames_seen);

        if !video.fullscreen {
            video.write_footer(
                &mut stdout,
                render_fps,
                current_time,
                duration,
                elapsed,
                start - started,
            )?;
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
