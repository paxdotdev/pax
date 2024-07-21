# Breakout 🧱

Breakout game with complex physics and stateful effects.

This demonstrates the ability of the large language model (LLM) both as a prototyper (complete first draft) and as a collaborator (subsequent revisions based on human input).

This is made possible by two key ingredients: (1) high-enough code intelligence and (2) a context window that is wide enough to accommodate _all_ of Pax's documentation and examples. (This is also known as in-context learning and few-shot prompting.)

The LLM was prompted using `prompt-v1.0.0.txt` in [`paxgen`](../../../scripts/paxgen/README.md), itself auto-generated from Pax docs and examples at commits:
- `c1cec6a3fa3b665d30cb9afafac7ac019aca84d9` for `pax-docs`;
- `8b10fc57e9503b3a08a53e687b9ee5d25555e48f` for `pax-engine`.

Note that the examples include `bouncing-balls`, a simpler prototype that tested the LLM's ability to implement simple collision detection. Although both `bouncing-balls` and `breakout` are part of the same commit & PR, `bouncing-balls` was made prior to `breakout` and was included in the LLM context when making `breakout`.

Run with:

```bash
./pax run --target=web
```