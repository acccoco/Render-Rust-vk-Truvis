set shell := ["powershell.exe", "-c"]

build-all:
	cargo build --all

# 编译着色器
shader:
	cargo run --bin shader-build

cxx:
	cargo run --bin cxx-build

cornell: shader
	cargo run --bin rt-cornell

sponza: shader
	cargo run --bin rt-sponza
