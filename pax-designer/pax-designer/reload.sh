# Path to the pax-cli file
PAX_CLI="../pax/target/debug/pax-cli"

pushd ../pax-designer
PAX_WORKSPACE_ROOT=../pax PAX_CORP_ROOT=../ $PAX_CLI build --target=web --libdev --verbose
popd