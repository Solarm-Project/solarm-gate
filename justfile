default:
	just --list

build:
	PKG_CONFIG_PATH="/opt/homebrew/opt/libarchive/lib/pkgconfig" cargo build -p pkgdev

serve:	
	cargo watch -x 'run -p forge -- --testing'

dep-install:
	cargo install cargo-watch
	cargo install cargo-edit