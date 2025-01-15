# AI-Discord-GameMaster

## Doc 

- [setup_discord](docs/setup_discord.md)
- [setup_openAI](docs/setup_openAI.md)

## Quality

- cargo check
- cargo clippy
- cargo fmt -- --check
- cargo audit
- cargo udeps


//FIXME

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



## Release

- update version in `Cargo.toml`
- build
```
cargo build --release --target=x86_64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-gnu
```
- prepare zip in ../release
- push
- create gitlab release with link to zip