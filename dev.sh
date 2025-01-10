#!/bin/bash

set -e

CARGO_TOML="Cargo.toml"

function bump_version() {
    # Read the version from Cargo.toml (only the first occurrence under [package])
    local current_version=$(awk '/\[package\]/ {flag=1} flag && /^version = / {print; exit}' "$CARGO_TOML" | sed -E 's/version = "([0-9]+\.[0-9]+\.[0-9]+)"/\1/')
    
    # Split version into major, minor, patch
    IFS='.' read -r major minor patch <<< "$current_version"

    # Determine which part to bump
    case $1 in
        patch)
            patch=$((patch + 1))
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        *)
            echo "Unknown version bump type: $1"
            exit 1
            ;;
    esac

    # Construct the new version
    local new_version="${major}.${minor}.${patch}"

    # Update the Cargo.toml file (only the first occurrence under [package])
    awk -v new_version="$new_version" '/\[package\]/ {flag=1} flag && /^version = / {sub(/"[0-9]+\.[0-9]+\.[0-9]+"/, "\"" new_version "\""); flag=0} {print}' "$CARGO_TOML" > "$CARGO_TOML.tmp" && mv "$CARGO_TOML.tmp" "$CARGO_TOML"

    echo "Version bumped to $new_version"
}

if [[ "$1" == "bump" ]]; then
    if [[ -z "$2" ]]; then
        echo "Error: Missing argument for bump (patch | minor | major)."
        exit 1
    fi
    bump_version "$2"
elif [[ "$1" == "build" ]]; then
    cargo build
elif [[ "$1" == "run" ]]; then
    cargo run
else
    echo "Unknown command: $1"
    echo "Usage: $0 bump (patch | minor | major)"
    echo "Usage: $0 build"
    echo "Usage: $0 run"
    exit 1
fi
