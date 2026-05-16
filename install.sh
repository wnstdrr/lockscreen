#!/usr/bin/env bash
set -euo pipefail

BIN_DIR="${BIN_DIR:-/usr/local/bin}"
MAN_DIR="${MAN_DIR:-/usr/local/share/man/man1}"

check_deps() {
    local missing=()
    for cmd in cargo i3lock; do
        command -v "$cmd" &>/dev/null || missing+=("$cmd")
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        echo "error: missing dependencies: ${missing[*]}" >&2
        exit 1
    fi
}

build() {
    echo "Building lockscreen..."
    cargo build --release
}

install_bin() {
    echo "Installing binary -> ${BIN_DIR}/lockscreen"
    install -Dm755 target/release/lockscreen "${BIN_DIR}/lockscreen"
}

install_man() {
    echo "Installing man page -> ${MAN_DIR}/lockscreen.1"
    install -Dm644 lockscreen.1 "${MAN_DIR}/lockscreen.1"
}

main() {
    cd "$(dirname "$0")"

    check_deps
    build
    install_bin
    install_man

    echo "Done. Run 'man lockscreen' or 'lockscreen --help' to get started."
}

main "$@"
