use anyhow::Result;
use std::ffi::CString;
use std::ptr;
use video_rs::ffmpeg::ffi::*;

fn ffmpeg_initialize() -> Result<()> {
    unsafe {
        avdevice_register_all();
    }
    Ok(())
}

pub fn ffprobe_get_fps(url: &str) -> Result<u64> {
    ffmpeg_initialize()?;

    let c_url = CString::new(url)?;
    let mut format_context = ptr::null_mut();

    unsafe {
        if avformat_open_input(
            &mut format_context,
            c_url.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        ) != 0
        {
            return Err(anyhow::anyhow!("Failed to open input"));
        }

        if avformat_find_stream_info(format_context, ptr::null_mut()) < 0 {
            return Err(anyhow::anyhow!("Failed to find stream info"));
        }

        let stream = (*format_context).streams;
        let video_stream_index = (0..(*format_context).nb_streams)
            .find(|&i| {
                let codec_parameters = (*(*stream.add(i as usize))).codecpar;
                (*codec_parameters).codec_type == AVMediaType::AVMEDIA_TYPE_VIDEO
            })
            .ok_or_else(|| anyhow::anyhow!("No video stream found"))?;

        let codec_parameters = (*(*stream.add(video_stream_index as usize))).codecpar;
        let frame_rate = (*codec_parameters).framerate;

        if frame_rate.den != 0 {
            Ok(frame_rate.num as u64 / frame_rate.den as u64)
        } else {
            Err(anyhow::anyhow!("Invalid frame rate"))
        }
    }
}

pub async fn ffprobe_get_duration(url: &str) -> Result<u64> {
    ffmpeg_initialize()?;

    let c_url = CString::new(url)?;
    let mut format_context = ptr::null_mut();

    unsafe {
        if avformat_open_input(
            &mut format_context,
            c_url.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        ) != 0
        {
            return Err(anyhow::anyhow!("Failed to open input"));
        }

        if avformat_find_stream_info(format_context, ptr::null_mut()) < 0 {
            return Err(anyhow::anyhow!("Failed to find stream info"));
        }

        let duration = (*format_context).duration;
        avformat_close_input(&mut format_context);

        if duration != AV_NOPTS_VALUE {
            Ok((duration as f64 / AV_TIME_BASE as f64) as u64)
        } else {
            Err(anyhow::anyhow!("Failed to get duration"))
        }
    }
}
