# Media Fixture Attribution

## Test-Videos Big Buck Bunny Files

- Retrieval URLs: the `https://test-videos.co.uk/vids/bigbuckbunny/...` URLs in
  `manifest.json`.
- Upstream page: <https://test-videos.co.uk/bigbuckbunny/mp4-h264>
- License evidence: Test-Videos describes the files as free test videos; the
  underlying Big Buck Bunny film is licensed under Creative Commons Attribution.
- Attribution: `(c) copyright 2008, Blender Foundation / www.bigbuckbunny.org`.
- Redistribution note: media binaries are downloaded by developers or CI and are
  not committed to this repository.

## Chromium Media Test Data

- Source repository: <https://chromium.googlesource.com/chromium/src/>
- Immutable download mirror: <https://github.com/chromium/chromium>
- Immutable revision: `98a04f617f22fdc3621924151b877b19b87cd5fd`.
- Base retrieval URL:
  `https://raw.githubusercontent.com/chromium/chromium/98a04f617f22fdc3621924151b877b19b87cd5fd/media/test/data/{filename}`
- License evidence: Chromium source files state that use is governed by a
  BSD-style license found in the Chromium `LICENSE` file.
- Attribution: Copyright The Chromium Authors.
- Retrieval note: `bear-1280x720_av_frag.mp4` is stored by the pinned Chromium
  revision as `bear-1280x720-av_frag.mp4`. The downloader saves it to the
  deterministic destination requested by the fixture manifest.
- Redistribution note: the revision-pinned raw mirror response is installed
  into an ignored fixture directory after exact integrity validation.

## Derived Fixtures

Derived fixtures are generated locally with `ffmpeg` from
`test-fixtures/source/bbb-h264.mp4` and deterministic subtitle inputs. They
inherit the Big Buck Bunny attribution for copied video streams. SRT text is
generated locally for testing.
