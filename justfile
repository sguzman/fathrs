set shell := ["bash", "-uc"]

default:
	@just --list

# Context
context:
	files-to-prompt --ignore target/ .git/ --markdown --line-numbers --extension yaml --extension yml --extension rs --extension toml --extension md . > ~/Downloads/all.txt

context-by-file:
	files-to-prompt --ignore target/ .git/ --markdown --line-numbers --extension rs . > ~/Downloads/rs.txt
	files-to-prompt --ignore target/ .git/ --markdown --line-numbers --extension md . > ~/Downloads/md.txt
	files-to-prompt --ignore target/ .git/ --markdown --line-numbers --extension toml . > ~/Downloads/toml.txt
	files-to-prompt --ignore target/ .git/ --markdown --line-numbers --extension yaml --extension yml . > ~/Downloads/yaml.txt

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
all: fmt typos links validate test build
