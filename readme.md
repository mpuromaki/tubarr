# Tubarr

Tubarr is Sonarr-like software for managing and downloading (with yt-dlp) Youtube etc videos.
Idea is that users can either copy&paste interested videos for downloading, or that Tubarr
will track channels and automatically download videos.

## Requirements

- Local installation of ```ffmpeg```
- Local installation of ```yt-dlp```
- Built binary of tubarr

## Running

- Run Tubarr binary: ```./tubarr```
- Answer configuration questions on the first startup.
- (All data is stored in the sqlite database, in system specific configuration folder.)
- Open Tubarr UI with web browser: http://127.0.0.1:8000/

## Hard truths

Let's start by stating the hard truths about this software. I am not a programmer. I do not
have any time allocated for this software. I know this software is very unelegant, it's by
design. I want to really understand how it works. Simplicity is the name of the game.

### Requests

Whether they're bug reports or feature request I will likely not have time to respond in
any kind of timely matter. Sorry. This is mainly my software for my needs, if it helps
you, then great! 

But I'm not against fixing issues. Atleast for now.

### Contributing

Feel free to contribute, but read those hard truths first. If I don't understand your
pull request, I'm not going to pull it.

## Roadmap

- [x] Basic setup of database, webserver, background tasks
- [x] Web UI
- [x] Downloading single youtube videos
- [/] Tracking youtube channels
- [/] Linux support
- [ ] Windows support
- [ ] Downloading tracked channels automatically
- [ ] Users handling
- [ ] Security, API-keys, etc
- [ ] Beautiful Web UI