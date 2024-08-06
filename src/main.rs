use clap::Parser;
use crossterm::event::{read, Event, KeyCode, KeyModifiers};
use crossterm::{
    cursor::{self, MoveTo},
    execute,
    style::{self},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, BufWriter, Write};
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
    let (frames_recv, seek_tx) = video.fetch_video(video.hw_accel.clone()).await.unwrap();
    let (render_tx, render_recv) = unbounded_channel::<(Frame, DurationType)>();

    let frames_recv = Arc::new(RwLock::new(frames_recv));

    let mut stdout = io::stdout();

    execute!(stdout, Clear(ClearType::All))?;
    execute!(stdout, cursor::Hide)?;

    // Spawn a task to handle signal input
    tokio::spawn(handle_signal_input());

    // Spawn a task to render video frames
    let handle_render = tokio::spawn(handle_render(
        video,
        seek_tx,
        render_recv,
        frames_recv.clone(),
    ));

    // Forward frames to the render task
    loop {
        let mut frames_recv = frames_recv.write().await;
        let data = match frames_recv.recv().await {
            Some(data) => data,
            None => break,
        };

        drop(frames_recv);

        render_tx.send(data).unwrap();
    }

    let _ = handle_render.await?;

    Ok(())
}

fn end() {
    terminal::disable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(
        stdout,
        cursor::Show,
        style::ResetColor,
        MoveTo(0, 0),
        Clear(ClearType::All)
    )
    .unwrap();
    exit(0);
}

// Handle signal to quit the application
async fn handle_signal_input() {
    tokio::signal::ctrl_c().await.unwrap();
    end();
}

// Drain the receiver channels
// A bit buggy though
async fn drain_receiver(recv: &mut UnboundedReceiver<(Frame, DurationType)>) {
    while recv.try_recv().is_ok() {}
}

// Render video frames to the terminal
async fn handle_render(
    mut video: Video,
    seek_tx: UnboundedSender<i64>,
    render_recv: UnboundedReceiver<(Frame, DurationType)>,
    frames_recv: Arc<RwLock<UnboundedReceiver<(Frame, DurationType)>>>,
) -> anyhow::Result<()> {
    let started = Instant::now();
    let std_frame_time = Duration::from_micros(1_000_000 / video.fps);
    let frames_seen = Arc::new(RwLock::new(0));
    let mut frame_times: Vec<Instant> = vec![];
    let render_recv = Arc::new(RwLock::new(render_recv));

    let mut stdout = BufWriter::new(io::stdout());

    let (mut last_width, mut last_height) = terminal::size()?;

    terminal::enable_raw_mode()?;

    let frames_seen_copy = frames_seen.clone();
    let render_revc_copy = render_recv.clone();

    tokio::spawn(async move {
        loop {
            let ev = read();
            if let Ok(Event::Key(event)) = ev {
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

                        let mut render_recv = render_revc_copy.write().await;
                        let mut frames_recv = frames_recv.write().await;

                        let new_frames = ((current_time + 5.0) * (video.fps as f32)) as u64;

                        *frames_seen = new_frames;

                        drain_receiver(&mut render_recv).await;
                        drain_receiver(&mut frames_recv).await;

                        drop(render_recv);
                        drop(frames_recv);
                        drop(frames_seen);
                    }

                    if event.code == KeyCode::Char('k') {
                        let mut frames_seen = frames_seen_copy.write().await;
                        let current_time = *frames_seen as f32 / video.fps as f32;

                        seek_tx
                            .send((current_time * 1000.0 - 5000.0) as i64)
                            .unwrap();

                        let mut frames_recv = frames_recv.write().await;
                        let mut render_recv = render_revc_copy.write().await;

                        let new_frames = ((current_time - 5.0) * (video.fps as f32)) as u64;

                        *frames_seen = new_frames;

                        drain_receiver(&mut render_recv).await;
                        drain_receiver(&mut frames_recv).await;

                        drop(render_recv);
                        drop(frames_recv);
                        drop(frames_seen);
                    }
                }
            }
        }
    });

    // while let Some((frame, duration)) = render_recv.recv().await {
    loop {
        let mut render_recv = render_recv.write().await;

        let (frame, duration) = match render_recv.recv().await {
            Some(data) => data,
            None => break,
        };

        drop(render_recv);

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
