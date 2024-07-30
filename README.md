[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]

<br />
<div align="center">
  <a href="https://github.com/dan-online/window">
    <img src="assets/window-logo-round.webp" alt="Window logo" width="80" height="80">
  </a>

<h3 align="center">window</h3>
  <p align="center">
    effortlessly watch videos directly in your terminal</br> with support for local, remote or YouTube videos
    <br />
    <br />
    <a href="https://github.com/dan-online/window/issues">Report Bug</a>
    ·
    <a href="https://github.com/dan-online/window/issues">Request Feature</a>
  </p>
</div>

## About The Project

Window is a terminal-based video player that allows you to watch videos directly in your terminal. It supports local, remote, and YouTube videos assisted by hardware accelerated decoding

![window](assets/demo.gif)

### Features

- **Variety of Video Sources**
    - **Local File**: Seamlessly play videos stored on your local machine.
    - **Remote File**: Stream videos directly from remote URLs, ensuring flexibility and convenience.
    - **YouTube Video (using yt-dlp)**: 
        - **Video on Demand (VOD)**: Access and play YouTube videos on demand, with support for various formats and qualities.
        - **Live**: Watch live streams from YouTube in real-time, providing a unique and dynamic viewing experience.
    - **Live Stream**: Support for various live streaming protocols, allowing you to watch live broadcasts from different platforms and sources.

- **Hardware Acceleration**
    - **Enhanced Performance**: Leverages hardware acceleration to optimize decoding of video streams, reducing CPU usage and providing smoother playback.
    - **Multi-Platform Support**: Compatible with a wide range of hardware, ensuring optimal performance across different devices and operating systems.


## TODO

- [ ] Error handling
- [x] Full screen mode
- [ ] Resizing
- [ ] Tests
- [ ] Audio???
- [ ] Subtitles???
- [ ] Better color accuracy

## Getting Started

> **Note:** Window requires ffmpeg libraries to be installed, for more information check [here](https://github.com/zmwangx/rust-ffmpeg/wiki/Notes-on-building#dependencies)

> **Note:** Window requires yt-dlp installed in order to use the youtube feature. You can install it by running `pip install yt-dlp`.

### Docker

```bash
$ docker run -it --rm danonline/window:latest --help
$ docker run -it --rm danonline/window:latest "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

### Bin

Check out the [releases](https://github.com/dan-online/window/releases) page for the latest binaries.

## License

Distributed under the MIT License. See [`LICENSE`](https://dancodes.mit-license.org) for more information.

## Contact

DanCodes - <dan@dancodes.online>

Project Link: [https://github.com/dan-online/window](https://github.com/dan-online/window)

[contributors-shield]: https://img.shields.io/github/contributors/dan-online/window.svg?style=for-the-badge
[contributors-url]: https://github.com/dan-online/window/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/dan-online/window.svg?style=for-the-badge
[forks-url]: https://github.com/dan-online/window/network/members
[stars-shield]: https://img.shields.io/github/stars/dan-online/window.svg?style=for-the-badge
[stars-url]: https://github.com/dan-online/window/stargazers
[issues-shield]: https://img.shields.io/github/issues/dan-online/window.svg?style=for-the-badge
[issues-url]: https://github.com/dan-online/window/issues
[license-shield]: https://img.shields.io/github/license/dan-online/window.svg?style=for-the-badge
[license-url]: https://github.com/dan-online/window/blob/master/LICENSE.txt
