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
- cargo deny check
- cargo spellcheck

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