# AI-Discord-GameMaster

This project implements a Discord bot that acts as a game master, leveraging MML to respond to player commands and drive a role-playing game experience. The bot uses a text completion API to generate responses based on player input. In addition, it can read the answer aloud in discord voice channel.

This is a quick project to learn rust, scope is small.

## Usage

After setting up the dependencies and running the bot, players can use the following commands:

* `!help`: Displays a list of available commands.
* `!join`: Joins the voice channel the user is currently in.
* `!leave`: Leaves the current voice channel.
* `!play <url/search term>`: Plays an audio track from a provided URL or searches YouTube for a track.
* `!stop`: Stops currently playing audio.
* `!report <text>`: Sends a detailed description or prompt to the AI Game Master.  The response will be displayed in the text channel.  Use reactions to have the bot read the response aloud.

## Setup + Doc 

- [setup_discord](docs/setup_discord.md)
- [setup_openAI](docs/setup_openAI.md)

## Dependencies

* `serenity`: Discord API interaction.
* `songbird`: Voice chat functionality.
* `tokio`: Asynchronous runtime.
* `reqwest`: HTTP client.
* `tracing`: Logging.

## Release

- update version in `Cargo.toml`
- quality checks (run for each cargo)
```bash
cargo update
cargo test
cargo bench
cargo check
cargo clippy
cargo fmt -- --check
##
## requires: rustup default nightly 
cargo udeps --all-targets
## requires git config (won't work in devcontainer)
cargo audit
## to reclaim disk space
cargo clean
```
- build
```bash
# rustup default stable 
cargo clean

# for target/x86_64-unknown-linux-gnu/release/ai-discord-gamemaster
VERSION=1.2.0-rc202502041611 cargo build --release --target=x86_64-unknown-linux-gnu
## requite install toolchain
## rustup target add x86_64-pc-windows-gnu
## sudo apt-get install mingw-w64 libopus-dev
VERSION=1.2.0-rc202502041611 cargo build --release --target x86_64-pc-windows-gnu
```
- prepare `.tar.gz` + `.zip` in ../release (including helper `.sh` + `.bat`)
- push
- create gitlab release with link to zip

## FIXME
- `cargo build --release --target x86_64-pc-windows-gnu` doesn't work anymore, no idea why. In `Cargo.toml`
```
[target.x86_64-pc-windows-gnu]
rustflags = [
    "-C",
    "link-args=-Wl,--allow-multiple-definition",
    "-lssp",
    "-lgcc",
    "-lpthread",
    "-L/usr/lib/gcc/x86_64-w64-mingw32/10-posix/",
]
linker = "x86_64-w64-mingw32-gcc"
```
- using `#![allow(deprecated)]` for serenity
- `cargo audit`: many issues like:
```
Crate:     tungstenite
Version:   0.18.0
Title:     Tungstenite allows remote attackers to cause a denial of service
Date:      2023-09-25
ID:        RUSTSEC-2023-0065
URL:       https://rustsec.org/advisories/RUSTSEC-2023-0065
Severity:  7.5 (high)
Solution:  Upgrade to >=0.20.1
Dependency tree:
tungstenite 0.18.0
└── tokio-tungstenite 0.18.0
    └── twilight-gateway 0.15.4
        └── songbird 0.4.6
            ├── text_completion 1.0.0
            │   ├── openai_api 1.0.0
            │   │   └── ai-discord-gamemaster 1.0.0
            │   └── discord 1.0.0
            │       └── ai-discord-gamemaster 1.0.0
            ├── discord 1.0.0
            └── ai-discord-gamemaster 1.0.0
```

## License

MIT