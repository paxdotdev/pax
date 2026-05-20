# Pain points

Example (structure is not rigid but this showcases the style of notes we should be gathering.

## 2026-05-19

Tried to do `x`, ran into `y`, solved it by `z`.  
Recommendations:  (e.g. document in pax-docs, document in API docs (inline comments), update AGENTS.md, create a more targeted example, consider redesigning APIs, etc.) 

## 2026-05-20

Mobile WebKit requires `DeviceOrientationEvent.requestPermission()` /
`DeviceMotionEvent.requestPermission()` to run inside a user gesture. The web
chassis originally handled that by injecting a global "Enable motion" button,
which made every Pax web app show gyro-specific UI on mobile.

Solved by keeping sensor startup in the web chassis but exposing
`window.paxRequestDeviceSensorPermissions()` for app-owned controls. The
`gyro-helper` example owns its visible `Enable motion` button and calls that hook
from its Pax `Button` handler, so other web apps do not inherit the prompt.

Recommendations: document the helper near `$gyro` / `$accel` web-target docs if
this becomes an officially supported developer-facing API, and consider a
typed Pax runtime API for requesting platform permissions instead of direct web
JS interop.
