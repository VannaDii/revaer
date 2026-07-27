# Media Fixture Attribution

## Test-Videos Big Buck Bunny Files

- Retrieval URLs: the `https://test-videos.co.uk/vids/bigbuckbunny/...` URLs in
  `manifest.json`.
- Fallback retrieval URLs: exact Internet Archive captures listed in
  `manifest.json` for Test-Videos entries whose primary URLs are intermittently
  unavailable.
- Upstream page: <https://test-videos.co.uk/bigbuckbunny/mp4-h264>
- License evidence: Test-Videos describes the files as free test videos; the
  underlying Big Buck Bunny film is licensed under Creative Commons Attribution.
- Attribution: `(c) copyright 2008, Blender Foundation / www.bigbuckbunny.org`.
- Redistribution note: media binaries are downloaded by developers or CI and are
  not committed to this repository.

## Matroska Official Test Files

- Repository: <https://github.com/ietf-wg-cellar/matroska-test-files>
- Files copied from `test_files/`: `test1.mkv`, `test2.mkv`, `test3.mkv`,
  `test4.mkv`, `test5.mkv`, and `test8.mkv`.
- License evidence: the repository README states that Big Buck Bunny and
  Elephant Dreams assets are licensed under Creative Commons Attribution and
  recommends the Big Buck Bunny attribution text above.
- Redistribution note: media binaries are cloned into a temporary workspace and
  copied into ignored fixture directories; they are not committed.

## Chromium Media Test Data

- Base retrieval URL:
  `https://chromium.googlesource.com/chromium/src/+/lkgr/media/test/data/{filename}?format=TEXT`
- License evidence: Chromium source files state that use is governed by a
  BSD-style license found in the Chromium `LICENSE` file.
- Attribution: Copyright The Chromium Authors.
- Retrieval note: `bear-1280x720_av_frag.mp4` is stored by current Chromium
  `lkgr` as `bear-1280x720-av_frag.mp4`. The downloader saves it to the
  deterministic destination requested by the fixture manifest.
- Redistribution note: responses are base64 encoded by Gitiles and decoded into
  ignored fixture directories.

## Derived Fixtures

Derived fixtures are generated locally with `ffmpeg` from
`test-fixtures/source/bbb-h264.mp4` and deterministic synthetic audio/subtitle
inputs. They inherit the Big Buck Bunny attribution for copied video streams.
Synthetic sine tones, silence, and SRT text are generated locally for testing.
