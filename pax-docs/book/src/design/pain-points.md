# Pain points — raw findings inbox

This is a temporary intake for findings encountered during work, not a reference
or a prerequisite for agents to read before starting a task.

Add concise, dated reports: what you tried, expected versus observed behavior,
reproduction steps or evidence, relevant target/revision, and any workaround or
related Linear issue. Link repeated reports instead of copying long journals.

Periodically triage the inbox: discard resolved or obsolete findings, preserve
ongoing issues and report details in Pax Core Linear tickets, and remove each
processed entry. Keep only exceptional proactive guidance in `AGENTS.md`.
Design exploration and specifications belong directly in Linear, not here.

Last triaged: 2026-10-05 — [PAX-972](https://linear.app/paxdev/issue/PAX-972/pay-down-the-pain-pointsmd-file).

## Untriaged findings

## 2026-10-06 — macOS display-link startup

A macOS example can open a blank window if CoreVideo reports
`CVDisplayLinkCreateWithCGDisplays error -6661` with an active display count of
zero. In the Scroll Matrix smoke test, relaunching after the display became
available restored rendering without source changes. Inspect the app's system
log before treating an idle blank window as a renderer regression. The current
display-link setup does not retry that startup failure automatically.
