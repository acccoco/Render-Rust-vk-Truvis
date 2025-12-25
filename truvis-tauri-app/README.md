# Tauri + React + Typescript

测试启动命令
`npm run tauri dev`  ，实际运行的内容：
- `npm run dev` 
- `cargo run --manifest-path src-tauri/Cargo.toml` 

正式发布：
`npm run tauri build` 
- `npm run dev`
- `cargo build --release`