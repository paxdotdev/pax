#!/bin/bash

pushd ../pax-compiler
cargo run -- run --target=web ../pax-example
