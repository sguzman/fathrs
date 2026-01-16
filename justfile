set shell := ["bash", "-uc"]

default:
	@just --list

# Build
build:
	cargo build

# Format
fmt:
	cargo fmt
	taplo fmt
	biome format --write .

# Validate
typos:
	typos --config typos.toml
links:
	lychee --config lychee.toml .
validate:
	taplo validate

# Test
test:
	cargo test

# Everything
all: build
