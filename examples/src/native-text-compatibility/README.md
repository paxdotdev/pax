# Native text compatibility fixture

A manual macOS regression fixture for native labels, document text, editing,
and accessibility during scrolling. The separate viewport-proximity example
remains the animation/performance workload. This fixture does not establish
iOS/iPadOS compatibility.

From the monorepo root:

```sh
cargo build -p pax-cli
PAX_WORKSPACE_ROOT="$PWD" ./target/debug/pax-cli run --libdev \
  --path examples/src/native-text-compatibility --target macos --release
```

Rebuild pax-cli after changing canonical Swift templates so the generated app
uses those changes. Use a window around 900 × 900 points for the checks below.

## Manual checks

1. Double-click a word in the plain label, move to the start with Command-Left,
   and extend the selection with Option-Shift-Right. Copy with Command-C and
   paste into the note. Its bound value should match. Repeat with the whole
   sentence, including Café and 🦊, pasted into the multiline editor.
2. Select text in the plain label and Markdown paragraph. Wheel over each:
   the containing list should scroll. Markdown should retain bold text and
   its link styling. Editable text may scroll within its own editing area.
3. Using an actual input method, clear the note and begin composition. For
   example, with Japanese–Romaji in Hiragana mode, type `nihonn`, convert with
   Space, and commit the candidate. Before committing, the preedit glyphs and
   underline should fit inside the empty editor, and the bound note should
   remain empty. After committing 日本, the bound note should match.
4. Begin another composition, scroll the containing list several pages away
   and back without changing focus, then cancel. The marked text should survive
   scrolling, and cancellation should preserve the previously committed value.
   Repeat the empty-editor check in the multiline editor. Pasting Japanese
   characters is not a substitute for this input-method test.
5. Enable VoiceOver and use its navigation/Item Chooser to reach labels and the
   distant “End marker: Magnolia”. Scroll away and back, checking that content
   remains discoverable. Disable VoiceOver and verify ordinary scrolling and
   selection still work. Restore any input-source or accessibility settings
   changed for testing.

## Recorded macOS pass — 2026-09-28

- Keyboard word selection, copy/paste, and full Unicode-sentence copy/paste
  passed. Both bound editor values updated correctly.
- Markdown styling and parent wheel scrolling passed. Real Japanese candidate
  conversion, commit, cancellation, and preservation while scrolling away/back
  passed before the empty-editor sizing fix.
- That IME check exposed vertically clipped preedit in an empty editor. The
  renderer now preserves the declared editing viewport and uses the declared
  font's line height for empty-editor measurement. A regression test verifies
  marked text and the viewport survive a geometry update; all 21 native tests
  pass in debug and release. Post-fix visual IME verification remains pending:
  the automation could not reliably switch to Japanese after relaunch.
- With VoiceOver off at the top, the accessibility tree exposed five nearby
  numbered labels and omitted the end marker. Enabling VoiceOver exposed all
  80 labels and the marker; all remained exposed while scrolling. Disabling it
  resumed culling (16 nearby labels at the tested bottom position).
- VoiceOver spoken output, navigation order, and cursor-driven reveal remain
  unverified: desktop automation stalled while inspecting VoiceOver's UI.
  Accessibility-tree presence alone is not a complete screen-reader pass.
- Temporary Japanese input/Dictation languages, input-menu and Globe-key
  preferences, and VoiceOver were restored to their original settings.

See the book's [selection and editing guide](../../../pax-docs/book/src/text-fonts-images.md#selection-and-editing)
for the public Text behavior.
