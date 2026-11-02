# Paxflix device previews

These are captures of the canonical `examples/paxflix` application,
not generated artwork. The native captures were made on 2026-10-14 from
checkout `9be02e878` (the website/startup work did not change Paxflix source).

- `macos.jpg`: macOS development host, catalog scrolled below the hero,
  captured with `pax-cli dev look`, JPEG quality 0.86, scale 0.65 (1436×937).
  Includes the real window title bar. The website adds the laptop silhouette.
- `ios.jpg`: iPhone 16 Pro simulator, iOS 18.2, initial catalog, captured with
  `simctl io screenshot --type=jpeg` (1206×2622). Status bar was temporarily
  set to 9:41, full Wi-Fi and battery, then restored. The website adds the
  phone bezel and island.
- `tablet.jpg`: website `/paxflix/` web route, captured on 2026-11-02 at a
  tablet-sized viewport. Only the application content below the 52px route
  header is retained; the website supplies the tablet bezel.

Regenerate using this checkout's CLI and the actual native apps:

```sh
source ~/.zshrc
nvm use
./target/debug/pax-cli run --path examples/paxflix --target macos --libdev
# In another terminal, after images/fonts settle and choosing the catalog view:
./target/debug/pax-cli dev look --path examples/paxflix --format jpeg \
  --quality 0.86 --scale 0.65 --output-dir /tmp/paxflix-macos
```

Stop the macOS session before staging the other target. Select an available
simulator UUID from `xcrun simctl list devices available`:

```sh
./target/debug/pax-cli run --path examples/paxflix --target ios \
  --ios-device simulator:DEVICE_UUID --libdev
xcrun simctl status_bar DEVICE_UUID override --time '9:41' \
  --dataNetwork wifi --wifiMode active --wifiBars 3 \
  --batteryState charged --batteryLevel 100
xcrun simctl io DEVICE_UUID screenshot --type=jpeg /tmp/paxflix-ios.jpg
xcrun simctl status_bar DEVICE_UUID clear
```

Inspect both captures before replacing the assets. Preserve their proportions;
`NativeImage` displays the captures without canvas color conversion;
`ImageFit::Paint` trims the small aspect-ratio difference inside the website's
stylized shells. All homepage devices are captures: there is no background
screenshot polling or hidden instance. To refresh the tablet image, run the
website, open `/paxflix/`, use an approximately 1024×871 CSS-pixel viewport,
wait for artwork to load, and crop off the 52px route header.
