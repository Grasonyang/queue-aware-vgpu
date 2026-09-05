#!/bin/bash

set -e

usage() {
    echo "Usage: $0 --install | --add NAME"
    echo
    echo "Options:"
    echo "  --install    Install k3d"
    echo "  --help       Show this help message"
    echo "  --add NAME   Add a k3d node with the given name"
}

install=false
add=false
name=""

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
        --add)
            if [[ $# -lt 2 ]]; then
                echo "Error: --add requires a node name" >&2
                exit 1
            fi
            add=true
            name="$2"
            shift 2
            ;;
        *)
            echo "Error: unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ "$install" == true ]]; then
    curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash
    exit 0
fi

if [[ "$add" != true ]]; then
    echo "Error: --add is required" >&2
    usage >&2
    exit 1
fi

k3d node create "$name"
