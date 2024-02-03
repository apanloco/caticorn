#!/usr/bin/env bash
./build.sh
cargo install basic-http-server
basic-http-server build
