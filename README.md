# AI-Discord-GameMaster


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