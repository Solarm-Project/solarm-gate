default:
	just --list

build:
	PKG_CONFIG_PATH="/opt/homebrew/opt/libarchive/lib/pkgconfig" cargo build -p pkgdev
