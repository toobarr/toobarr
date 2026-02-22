# toobarr

A media manager for YouTube collections using [yt-dlp](https://github.com/yt-dlp/yt-dlp).

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [ffmpeg](https://ffmpeg.org/)
- SQLite3 development libraries (`libsqlite3-dev` on Debian/Ubuntu, `sqlite` on macOS via Homebrew)

## Build

```
make build
```

Or with Docker:

```
make docker
make docker-up
```

## AI Usage

AI has been used to help to write some of the code in this project. 

## License

[GNU Affero General Public License v3.0](LICENSE)
