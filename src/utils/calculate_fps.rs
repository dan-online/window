use tokio::time::Instant;

pub fn calculate_fps(frame_times: &[Instant]) -> f64 {
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
