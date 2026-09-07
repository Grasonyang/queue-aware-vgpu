#!/bin/bash

set -e

usage() {
    echo "Usage: $0 --install"
    echo
    echo "Options:"
    echo "  --install    Install k9s"
    echo "  --help       Show this help message"
}

install=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --help)
            usage
            exit 0
            ;;
        --install)
            install=true
            shift
            ;;
        *)
            echo "Error: unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ "$install" == true ]]; then
    curl -sS https://webinstall.dev/k9s | bash
    exit 0
fi
