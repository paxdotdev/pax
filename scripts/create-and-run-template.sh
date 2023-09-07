#/bin/bash

# Usage: this script will use the `pax create` command in `pax-cli` to
#        generate the template project (pax-compiler/pax-create-template) into
#        the monorepo sandbox (pax-create-sandbox, which is .gitingored) and then
#        run the CLI's `pax run` in that repo.
#        This is intended to test the `pax create` flow, e.g. for iterating on the template project.

# Expected current working directory: pax monorepo root

rm -rf ./pax-create-sandbox
cargo build --manifest-path=pax-cli/Cargo.toml
target/debug/pax-cli create ./pax-create-sandbox --libdev
target/debug/pax-cli run --target=web --path=./pax-create-sandbox