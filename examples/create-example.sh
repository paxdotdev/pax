#!/bin/bash

read -p "Enter a name for the example: " NAME

# 2) Create proxy script to run example
cat <<EOL > "${NAME}.rs"
mod scripts;
use scripts::run_example;

fn main() {
    if let Err(error) = run_example() {
        eprintln!("Error: {}", error);
    }
}
EOL

current_dir=$(pwd)

pushd ../pax-cli > /dev/null

cargo build > /dev/null

../target/debug/pax-cli create "$current_dir/src/$NAME" --libdev > /dev/null

popd > /dev/null

echo "$NAME example created successfully in src/$NAME!"
