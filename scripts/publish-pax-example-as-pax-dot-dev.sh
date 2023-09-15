#!/bin/bash

# 1. Build pax-example
cargo build
cd pax-example
./pax build --target=web
cd ..

# 2. Patch in our GA snippet
perl -i -pe 'BEGIN{undef $/; $insert = `cat scripts/ga-snippet.partial.html`;} s|(<head>)|$1$insert|s' pax-example/.pax/build/Web/dist/index.html

# 3. Upload to S3; invalidate Cloudfront
aws --profile=pax s3 cp --recursive --acl=public-read pax-example/.pax/build/Web/dist/ s3://www.pax.dev/
aws --profile=pax cloudfront create-invalidation --distribution-id=E4XEPE4OLK651 --paths "/*"
