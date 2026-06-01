# Revaer Media Runtime Third-Party Notices

The default media runtime image installs media tools and codec libraries from
Alpine Linux packages. Package versions are resolved by the Alpine 3.23 package
index at image build time and must be captured in the per-image inventory.

## Core Runtime Tools

- FFmpeg and FFprobe: https://ffmpeg.org/
- ExifTool: https://exiftool.org/
- MediaInfo: https://mediaarea.net/en/MediaInfo
- MKVToolNix: https://mkvtoolnix.download/
- Bento4: https://www.bento4.com/

## Codec, Subtitle, Font, And Protocol Components

- x264: https://www.videolan.org/developers/x264.html
- x265: https://www.x265.org/
- dav1d: https://code.videolan.org/videolan/dav1d
- Opus: https://opus-codec.org/
- Vorbis and Theora: https://xiph.org/
- libass: https://github.com/libass/libass
- GnuTLS: https://www.gnutls.org/
- DejaVu fonts: https://dejavu-fonts.github.io/

## Runtime Policy

- The default image must not include components that require FFmpeg
  `--enable-nonfree`.
- Revaer invokes GPL media tools through process boundaries and files.
- ExifTool is the only approved first-release interpreted utility exception.
- Third-party source, license, attribution, and package inventory evidence must
  be archived for each published image digest.
