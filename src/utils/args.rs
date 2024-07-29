use clap::Parser;
use serde::Serialize;
use video_rs::hwaccel::HardwareAccelerationDeviceType;

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CharacterMode {
    #[default]
    Blocks,
    Dots,
    Ascii,
    AsciiExtended,
    Numbers,
    Unicode,
}

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScaleMode {
    #[default]
    Fit,
    Stretch,
}

// Hardware acceleration device type but clap compatible
#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareAcceleration {
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
    pub fn to_video_rs(&self) -> Option<HardwareAccelerationDeviceType> {
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
#[command(version, author, about, long_about = None)]
pub struct Args {
    pub url: String,

    #[clap(short, long, default_value = "2")]
    pub pixel_clear_distance: Option<u16>,

    #[clap(short, long, default_value = "blocks")]
    pub mode: Option<CharacterMode>,

    #[clap(short, long, default_value = "fit")]
    pub scale: Option<ScaleMode>,

    #[clap(short, long, default_value = "true")]
    pub cap_framerate: Option<bool>,

    #[clap(long, default_value = "none")]
    pub hw_accel: Option<HardwareAcceleration>,

    #[clap(long, short, action)]
    pub fullscreen: bool,
}
