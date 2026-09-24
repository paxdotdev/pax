# PAX-984 local theme fixture

Synthetic draft content only. This fixture is a bounded evaluation, not the
production blog. The real 0.39.0 article comes from PAX-978 after editorial
approval. Nothing in this directory deploys a site.

Use **Zola 0.23.6** and the **Tabi v5.0.0** theme. The workstation's existing
Zola 0.19.2 is too old for this theme. Put the tagged theme at `themes/tabi` in
a temporary copy of this fixture; retain its MIT license. Do not commit the
theme's demonstration content or generated output as part of this scope.

For the temporary evaluation already prepared on this workstation:

```sh
/private/tmp/pax-984-scope/zola/zola \
  --root /private/tmp/pax-984-scope/site build --drafts \
  --base-url http://127.0.0.1:8094 \
  --output-dir /private/tmp/pax-984-scope/output-preview
python3 -m http.server 8094 --bind 127.0.0.1 \
  --directory /private/tmp/pax-984-scope/output-preview
```

Run the server separately if port 8094 is not already serving the delivered
preview. Open `http://127.0.0.1:8094/pax-0-39-0/` or the index. For a fresh
evaluation, copy this directory to a new temporary site and place the tagged
Tabi source under its `themes/tabi/`; substitute that site's path above.

To test URL generation, use `--base-url https://blog.pax.dev` and, separately,
`--base-url https://www.pax.dev/blog`, with distinct output directories.
Zola emits relative files such as `pax-0-39-0/index.html` in both cases. The
hosting layer determines whether that tree is served at `/` or `/blog/`.

The fixture uses only local CSS/head overrides. All posts are drafts and every
page includes `noindex, nofollow`; do not publish fixture feeds or output.
Canonical links, feeds and aliases intentionally use the selected base URL.
This proves template composition and static publishing primitives, not Pax
embedding, final fonts, production caching, or newsletter migration.
