#!/bin/bash
# Patch pax-compiler and pax-cli cargo.toml
PAX_CLI_CARGO_TOML="../pax/pax-cli/Cargo.toml"
PAX_COMPILER_CARGO_TOML="../pax/pax-compiler/Cargo.toml"
DEPENDENCY="pax-designtime = { path=\"../../pax-designtime\", optional=true }"
PAX_COMPILER_FEATURE="designtime = [\"pax-designtime\"]"
PAX_CLI_FEATURE="designtime = [\"pax-compiler/designtime\"]"


# Escape brackets for grep pattern
ESCAPED_PAX_COMPILER_FEATURE=$(echo "$PAX_COMPILER_FEATURE" | sed 's/[][\/^$.*]/\\&/g')
ESCAPED_PAX_CLI_FEATURE=$(echo "$PAX_CLI_FEATURE" | sed 's/[][\/^$.*]/\\&/g')

# Function to check if Cargo.toml exists
check_cargo_toml_exists() {
    if [ ! -f "$1" ]; then
        echo "Cargo.toml not found at $1"
        exit 1
    fi
}

# Function to add dependency if not exists
add_dependency() {
    local cargo_toml=$1
    if ! grep -q 'pax-designtime' "$cargo_toml"; then
        # Insert the dependency followed by a newline
        sed -i '' -e "/\[dependencies\]/a\\
$DEPENDENCY
" "$cargo_toml"
    else
        echo "Dependency already exists in $cargo_toml."
    fi
}

# Function to append feature if not exists
append_feature() {
    local cargo_toml=$1
    local feature=$2
    local escaped_feature=$3
    if grep -q "\[features\]" "$cargo_toml"; then
        if ! grep -q "$escaped_feature" "$cargo_toml"; then
            awk -v feature="$feature" '/^\[features\]/{p=1;print;next} p&&!NF{print feature; p=0} {print}' "$cargo_toml" > tmpfile && mv tmpfile "$cargo_toml"
        else
            echo "Feature already exists in $cargo_toml."
        fi
    else
        echo "[features]" >> "$cargo_toml"
        echo "$feature" >> "$cargo_toml"
    fi
}

# Function to remove added dependency
remove_dependency() {
    local cargo_toml=$1
    # Using a more flexible pattern match
    sed -i '' -e '/pax-designtime = { path="..\/..\/pax-designtime", optional=true }/d' "$cargo_toml"
}

# Function to remove added feature
remove_feature() {
    local cargo_toml=$1
    local escaped_feature=$2
    
    sed -i '' -e "/$escaped_feature/d" "$cargo_toml"
}

add_designtime_dependency() {
    check_cargo_toml_exists "$PAX_COMPILER_CARGO_TOML"
    check_cargo_toml_exists "$PAX_CLI_CARGO_TOML"


    add_dependency "$PAX_COMPILER_CARGO_TOML"
    append_feature "$PAX_COMPILER_CARGO_TOML" "$PAX_COMPILER_FEATURE" "$ESCAPED_PAX_COMPILER_FEATURE"

    append_feature "$PAX_CLI_CARGO_TOML" "$PAX_CLI_FEATURE" "$ESCAPED_PAX_CLI_FEATURE"

    echo "Added designtime dependency to pax-cli and pax-compiler."
}

remove_designtime_dependency() {
    check_cargo_toml_exists "$PAX_COMPILER_CARGO_TOML"
    check_cargo_toml_exists "$PAX_CLI_CARGO_TOML"
    
    remove_dependency "$PAX_COMPILER_CARGO_TOML"
    remove_feature "$PAX_COMPILER_CARGO_TOML" "$ESCAPED_PAX_COMPILER_FEATURE"
    remove_feature "$PAX_CLI_CARGO_TOML" "$ESCAPED_PAX_CLI_FEATURE"

    echo "Removed designtime dependency from pax-cli and pax-compiler."
}
