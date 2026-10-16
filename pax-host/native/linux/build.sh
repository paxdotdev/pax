#!/usr/bin/env bash
set -euo pipefail
output=''
configuration=Debug
while (($#)); do
    case "$1" in
        --output-directory) output=$2; shift 2;;
        --configuration) configuration=$2; shift 2;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2;;
    esac
done
[[ -n $output && $output = /* ]]
[[ $configuration = Debug || $configuration = Release ]]
cd "$(dirname "$0")"
pkg-config --atleast-version=4.14 gtk4
mkdir -p "$output"
optimization=-O1
[[ $configuration != Release ]] || optimization=-O2
cc -std=c11 "$optimization" -g -Wall -Wextra -Werror -fPIC -fvisibility=hidden \
    -shared host.c native-scene.c gpu-import.c assets.c -o "$output/libpax_host.so" \
    $(pkg-config --cflags --libs gtk4 epoxy json-glib-1.0 fontconfig) -lm -ldl
printf '%s\n' "$output/libpax_host.so"
