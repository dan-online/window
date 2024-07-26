use youtube_dl::YoutubeDl;

pub fn get_youtube_video_from_url(url: &str) -> anyhow::Result<(String, u64, String)> {
    let output = YoutubeDl::new(url)
        .socket_timeout("15")
        .run()?
        .into_single_video()
        .ok_or("No video found")
        .map_err(|e| anyhow::anyhow!(e))?;

    println!("{:?}", output.title);

    let title = output
        .title
        .ok_or("No title found")
        .map_err(|e| anyhow::anyhow!(e))?;

    let output = output
        .formats
        .ok_or("No formats found")
        .map_err(|e| anyhow::anyhow!(e))?
        .into_iter()
        .filter(|f| f.vcodec.clone().unwrap_or_default().contains("avc"))
        .max_by_key(|f| (f.height.unwrap_or(0.0) + f.fps.unwrap_or(0.0)) as u64)
        .ok_or("No suitable format found")
        .map_err(|e| anyhow::anyhow!(e))?;

    let video_url = output
        .url
        .ok_or("No video URL found")
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok((video_url, output.fps.unwrap_or(30.0) as u64, title))
}
