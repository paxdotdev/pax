# CLI Telemetry
<!-- summary: What minimal Pax CLI telemetry contains and how to disable it. -->
<!-- tags: cli, telemetry, privacy -->

Pax uses a small amount of CLI telemetry to understand whether the tooling works
and whether installations return to build applications. The first
public CLI command on an installation prints a notice and sends no telemetry.
Later public commands enable telemetry by default.

Telemetry contains only:

- a random installation identifier stored by the CLI;
- the CLI version;
- the host operating-system family and CPU architecture;
- the top-level command family and, when applicable, its Pax target;
- whether a finished command succeeded or failed;
- whether a `pax-cli run` session reached its target-specific ready point; and
- approximate city, region, and country derived server-side from the connection
  IP.

The CLI does not add an IP address or location field to its telemetry payload.
Like any HTTPS service, the telemetry service sees the connection IP while
handling the request. It passes that address to Mixpanel only as reserved,
transient geolocation input. Mixpanel derives the approximate city, region, and
country and [discards the IP before ingesting the event][mixpanel-geolocation].
Pax does not log or persist the raw address. The coarse location is used only in
aggregate to guide product, content, outreach, and marketing investment.

A `run` that reaches its target-specific ready point emits only the ready
event, never a second event when it exits. Without observed readiness, a
finished `run` emits only its command outcome. In particular, iOS/iPadOS release
runs have no readiness channel: a clean exit is a successful command outcome,
not evidence of activation. Activation counts only ready events.

Pax does not collect source code, filenames, project paths, project names,
project content, command arguments, error text, account information, locale, or
hardware-derived identifiers.

Events are sent on a bounded best-effort basis by a short-lived background copy
of the CLI. They are not persisted, durably queued, or retried. A telemetry
failure never changes command behavior or exit status, and the foreground CLI
does not wait for network delivery. The worker may finish after the command has
returned to the prompt. Each request has a two-second timeout; once started,
the worker has a five-second deadline for input handling, local setup and
delivery. If worker launch or delivery fails, that event is silently dropped.

The worker rechecks consent immediately before sending. Ordinary commands and
telemetry status do not wait for in-flight network requests. An explicit
`telemetry off` disables future sends first, then waits up to 2.5 seconds for
any already-authorized requests to finish before reporting success. If that
wait times out, it reports an error but telemetry remains disabled.

## Controls

Use these commands to inspect or change the persistent setting:

```sh
pax-cli telemetry status
pax-cli telemetry off
pax-cli telemetry on
```

`pax-cli telemetry on` creates a fresh random installation ID immediately but
sends no event itself. The next public command may send telemetry. Turning
telemetry off and back on therefore starts a new installation identity.

Set `PAX_TELEMETRY=off` or `DO_NOT_TRACK=1` to disable telemetry for a process.
Telemetry is also disabled by default when `CI=1` or `CI=true` is present.

The local setting is stored under the operating system's Pax application-state
directory:

- macOS: `~/Library/Application Support/Pax/telemetry`
- Windows: `%LOCALAPPDATA%\Pax\telemetry`
- Linux: `$XDG_STATE_HOME/pax/telemetry`, or `~/.local/state/pax/telemetry`

The random installation identifier represents this local CLI installation and
is not tied to a person, account, hardware fingerprint, or project. Turning
telemetry off removes that identifier. Turning it on again creates a new one
immediately without sending an event.

## Update checks

CLI update checking is functional and separate from telemetry. It continues
when telemetry is off and never includes the installation identifier. Like any
HTTPS request, the update service sees the connection IP while handling the
request. Under the server contract, Pax does not retain that raw IP, derive
coarse location for the update check, or turn the update request into a
telemetry event. Update checks run in a background thread and never delay CLI
exit; an update notice is shown only if the result is already available.

[mixpanel-geolocation]: https://docs.mixpanel.com/docs/tracking-best-practices/geolocation
