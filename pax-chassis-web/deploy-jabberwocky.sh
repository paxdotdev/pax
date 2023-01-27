#!/bin/zsh

set -e

rm -rf pax-dev-harness-web/dist
wasm-pack build --release -d pax-dev-harness-web/dist
pushd pax-dev-harness-web
yarn build
cp index.html dist
popd

aws --profile=inclination s3 sync --acl=public-read ./pax-dev-harness-web/dist/ s3://static.pax.rs/jabberwocky/
find ./pax-dev-harness-web/dist/ | grep -i .wasm$ | xargs -I {} aws s3 cp --profile=inclination --acl=public-read --content-type=application/wasm {} s3://static.pax.rs/jabberwocky/

aws --profile=inclination cloudfront create-invalidation --distribution-id=E1MV9F4RQWI8Z9 --paths "/*"