# PLC Engineering Simulator — Phase 2 Preview Guide

This guide describes the simulator as it is available during Phase 2. It is an
offline, educational, **virtual-only** PLC engineering environment. It is not
accepted or released software yet, and it cannot connect to real controllers,
industrial networks, or physical equipment.

## Open the current preview

When the local preview server is running on this computer, open:

<http://127.0.0.1:43180/>

That address is local to this computer and is not a public Internet link. If it
does not load, the temporary preview server is no longer running; use the
native build path below or ask the project operator to restart the preview.

The browser preview is intended for exploring the engineering and virtual
runtime experience. Browser file access can be unavailable by design, so you
can always create and exercise a project in the current tab, but opening and
saving a `.vlabproj` file requires the approved native Windows shell.

## A quick first session

1. Wait for the header to show a ready core, then optionally select **Verify
   local foundation**. A healthy local result is expected.
2. Enter a project name and select **Create**.
3. Use **Add engineering object** and the project tree to make a virtual
   network, controller, rack, and virtual I/O modules. The runtime panel stays
   closed until the project has a valid fictional controller configuration.
4. Under the controller, add logic and data. The current workbench supports
   LAD, FBD, and SCL program blocks; function blocks/functions; global and
   instance data blocks; named types; tags; watch tables; and trace
   configurations.
5. Select **Runtime & commissioning** at the bottom of the workbench. Use
   **Build**, **Power on**, **Preview load**, then inspect the exact virtual
   download preview before selecting **Commit load**.
6. Select **Go online** to open an internal simulator session, then use
   **RUN** and **Scan +1** to execute deterministic virtual scans.
7. In the runtime panel, inspect probes and watch tables, set a virtual input,
   modify a value once, apply/remove a virtual force, and arm a trace. These
   actions affect only the virtual controller state in this project.
8. Stop the virtual CPU, choose **Capture snapshot**, make a change, then
   choose **Restore snapshot**. Select **Verify replay** to independently
   check the captured deterministic replay and display its receipt.

## Useful learning flows

### Logic and I/O behavior

Create a digital input and output module, add an input tag and output tag, then
author a simple LAD/FBD/SCL program. Build, load, run, change the virtual input
with **Set raw**, and advance a scan. Compare the probe's natural and effective
values to see how the logic responded.

### Monitoring, modification, forces, and traces

Add a watch table and trace configuration to the controller. After going
online, start monitoring, arm the trace, and advance scans. Use **Modify** for
a one-time virtual value change or **Force** to hold an effective virtual value;
use **Remove force** when finished. The UI marks forced values so the source of
the result remains visible.

### Safe fault and diagnostic exploration

Project and runtime diagnostics are navigable: selecting a diagnostic returns
to the relevant authored object when one is available. The simulator also has
a virtual watchdog/fault path for teaching purposes. A fault affects only the
in-memory virtual controller and produces a causal runtime diagnostic; it does
not affect any computer, PLC, or external device.

### Project editing

The project tree supports object selection, creation, rename/property editing,
duplicate-with-new-identity, deletion, and undo/redo. The status bar shows
whether project mutations have made the build stale, as well as a project hash
and a clear **Virtual only** boundary.

## Saving and reopening

In the native Windows shell, use **Save** or **Save as** to write a
simulator-native `.vlabproj` file. The product restricts this to approved local
project-file access and verifies the write before treating a save as complete.
Use **Open project** from the home screen to reopen that file. A reopened
project starts with a fresh powered-off virtual runtime; rebuild and load it
again before running it.

The temporary browser preview is deliberately not a general-purpose file
manager. If it says local file grants are unavailable, that is expected: the
current session remains usable, but persistent file open/save is disabled.

## Keyboard shortcuts

- `Ctrl+S` — save; uses Save As if the project does not yet have a file grant.
- `Ctrl+Shift+S` — Save As.
- `Ctrl+Z` — undo the last committed project change.
- `Ctrl+Y` — redo a reverted project change.
- `Delete` — delete the selected non-root object.

## Current boundaries and intentional limitations

- This is fictional virtual hardware only. There is no driver, protocol,
  device discovery, physical controller, industrial communication, deployable
  export, or live-machine download capability.
- “Go online” means online with the simulator's internal virtual controller,
  never a networked PLC.
- The preview is a Phase 2 implementation candidate. Its engineering features
  are available for exploration, but Phase 2's strict exact-candidate evidence
  and acceptance gate are still separate from normal product use.
- Browser preview persistence may be unavailable. Native project-file use is
  Windows-first and intentionally fail-closed for unsafe, remote, removable,
  redirected, provider-backed, or special targets.

## Troubleshooting

| What you see | What to do |
| --- | --- |
| The preview address does not open. | The temporary local server has stopped. Ask for the preview to be restarted, or launch the native build following the project workflow. |
| **Create** is disabled. | Wait for the core status to finish starting and make sure the project name is not blank. |
| Build/runtime buttons are disabled. | Read the Diagnostics panel. Create and configure one valid fictional controller, then resolve any blocking diagnostics. |
| **Go online**, **RUN**, or **Scan +1** is disabled. | Follow the sequence: Build → Power on → Preview load → Commit load → Go online → RUN. |
| Open/save is unavailable in the preview. | This is expected in browsers without the approved file broker. Explore in-session, or use the native Windows shell for `.vlabproj` persistence. |
| A close dialog appears. | Choose Cancel to keep working, Save and close to retain a verified save, or Discard to abandon unsaved changes. |
| A command reports “not completed.” | The application failed closed. Read the visible diagnostic/message, correct the project state, and retry; do not attempt to bypass the boundary. |

## Native build path for project operators

The exact native build and evidence workflow is documented in
[`tools/phase2/NATIVE_E2E_WORKFLOW.md`](tools/phase2/NATIVE_E2E_WORKFLOW.md).
It is deliberately separate from ordinary exploration because it is used for
strict Phase 2 verification, not as a way to reach physical hardware.
