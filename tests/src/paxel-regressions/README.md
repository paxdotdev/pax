# PAX-995 regression fixture

From the monorepo root, run the debug fixture:

```sh
cargo run -p pax-cli -- run --path tests/src/paxel-regressions --target web
```

Build the release fixture and serve its emitted bundle:

```sh
cargo run -p pax-cli -- build --path tests/src/paxel-regressions --target web --release
python3 -m http.server 62310 --bind 127.0.0.1 --directory tests/src/paxel-regressions/.pax/build/release/web
```

Both modes should display:

- Selected: 10
- Arithmetic: true
- Boolean: true
- Modulo: 0
- Skipped AND: false
- Skipped OR: true

Click **Change index**. The selected value must alternate between 10 and 20.
The handler only changes `index`; it deliberately leaves `items` untouched.
The two skipped boolean operands contain out-of-bounds accessors, so eager
execution fails instead of rendering these labels.

Boolean labels use ternaries because string-plus-boolean conversion is outside
this regression's scope. All tested expressions remain in the Pax template.
