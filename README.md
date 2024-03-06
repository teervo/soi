
# soi
**A command line music player for the pre-streaming era**

Soi plays music on your hard drive and not much more.

![Screenshot](https://user-images.githubusercontent.com/83108905/135654736-1bc540d5-f7d1-4fb6-bc1b-874f243a1d00.png)

## Features

- Support for pretty much any file format you can throw at it, thanks to the GStreamer backend
- Gapless playback
- Doesn't spit out errors when encountering .log/.cue files etc.

## Keyboard shortcuts

- `k` or up arrow: previous song
- `j` or down arrow: next song
- `h` or left arrow: seek backwards
- `l` or right arrow: seek forwards
- space: pause/continue playback
- `m`: mute/unmute
- `q`: quit program
- '?': show these shortcuts

## Dependencies
- GStreamer >= 1.8

## Installation
**With cargo**
```console
$ cargo install soi
```

**From git source**
```console
$ cargo install --git https://github.com/teervo/soi.git
```

