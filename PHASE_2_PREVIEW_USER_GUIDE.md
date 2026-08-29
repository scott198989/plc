# PLC Engineering Simulator — Step-by-Step Phase 2 Preview Guide

This is a beginner-friendly guide to the software as it works today. The app is an offline learning simulator: every controller, rack, I/O point, value, and scan is virtual. Nothing in this app can connect to a real PLC, a machine, or an industrial network.

> **Important:** this is a Phase 2 implementation preview, not a finished or accepted release. You can safely explore the features below, but some advanced features are intentionally not available yet.

## 1. Open the preview

On this computer, open the local preview link:

<http://127.0.0.1:43180/>

This link works only on this computer while the local preview is running. It is not a public website. If the page will not open, the preview server has stopped; ask the project operator to restart it.

When the opening page appears, wait for the top-right status to say **Core plc-engineering-core@0.2.0** (or another ready Core version). You can click **Verify local foundation** at the bottom of the page; a normal result says **HEALTHY**.

## 2. Create or open a project

### Create a new project

1. On the landing page, click the **Project name** box.
2. Type a name, such as `First Motor Lab`.
3. Click **Create**.
4. The engineering workbench opens. The left side is the project tree, the center is the editor, and the right side is the Properties panel.

Your new project is initially unsaved. That is normal.

### Open a saved project

1. On the landing page, click **Choose project file**.
2. Select a `.vlabproj` file that you previously saved from the native Windows version of the app.

If **Choose project file** is unavailable, the current browser preview does not have the approved local file access it needs. You can still create, edit, build, and run a temporary project in this tab. Saving and reopening files is available only through the approved native Windows shell.

## 3. Learn the workbench

| Where | What it does |
| --- | --- |
| Top bar | Provides Build, virtual power, virtual load, virtual online, RUN/STOP, scan, and replay controls. |
| Left project tree | Shows everything in the project. Click an item to select and edit it. |
| **+** button, labelled **Add engineering object** | Adds the items that are allowed below the selected item. |
| Center editor | Shows the selected item: a project summary, a LAD/FBD/SCL editor, a data/type member editor, or an information page. |
| Right Properties panel | Lets you rename the selected item, duplicate it with a new identity, or delete it. |
| Bottom tabs | **Diagnostics** explains problems. **Runtime & commissioning** is where you run the virtual controller. |
| Bottom status bar | Shows whether the project is built, the virtual runtime state, and the clear **Virtual only** safety reminder. |

### Basic editing controls

- Click an item in the tree, change its **Name** in the right panel, then click **Apply name**.
- Click **Duplicate with new identity** to make a distinct copy of the selected non-root item.
- Click **Delete object** to remove the selected non-root item.
- Use `Ctrl+Z` to undo and `Ctrl+Y` to redo a project change.
- Use `Delete` to delete the selected non-root item.

## 4. Add virtual hardware and I/O

This makes a fictional controller with virtual input and output points. It does not create or configure any real hardware.

1. In the left tree, click your project name (the very top item).
2. Click the **+** button (**Add engineering object**).
3. Click **Virtual network**. This is a data-only training network.
4. Click your project name again, click **+**, then click **Controller**.
5. Click **Controller** in the tree, click **+**, then click **Rack**.
6. Click **Local rack** in the tree. Click **+**, then add the modules you want:
   - **Digital input module** (`VDI16`)
   - **Digital output module** (`VDO16`)
   - **Analog input module** (`VAI4`)
   - **Analog output module** (`VAO4`)

The system assigns virtual slots and addresses. You do not need to type device addresses for this first exercise.

## 5. Add tags and a simple ladder program

Tags are the friendly names you use for virtual inputs, outputs, and memory. LAD (ladder logic) is the graphical logic editor.

1. Click **Controller** in the project tree.
2. Click **+**, then choose **Ladder organization block**. A block named **MainCycle** appears. It has one editable ladder rung.
3. Click **Controller** again, click **+**, then choose **Tag table**. A table named **PLC tags** appears.
4. Click **PLC tags**, click **+**, then choose **Input tag**. The app creates an `Input` BOOL tag with a virtual input address.
5. Click **PLC tags** again, click **+**, then choose **Output tag**. The app creates an `Output` BOOL tag with a virtual output address.
6. Click **MainCycle**. In the ladder rung, find the **Operand** list below the contact and select `InputValue` if it is not already selected.
7. In the **Contact** list, choose **Normally open**.
8. Find the **Operand** list below the coil and select `OutputValue` if it is not already selected.
9. In the **Coil** list, choose **Normal**.

The result is a simple, safe first automation: when the virtual input is true, the virtual output becomes true after a scan. You can think of it as a Start button turning on a virtual motor output. It is a virtual demonstration only.

### Important note about a true Start/Stop latch

A full motor latch normally needs multiple contacts, a parallel “seal-in” branch, and a Stop contact. The current Phase 2 LAD editor lets you edit the provided contact and coil (including normal, negated, set, and reset coil modes), but it does **not** yet let you add extra contacts or draw a parallel branch. Therefore, a complete real start/stop latch is intentionally not a current user feature. Do not mistake a virtual force or a Set coil for a complete production motor-control circuit.

The input-to-output exercise above is the complete, supported beginner LAD example in the current preview. It is the right way to learn the workflow: author → build → virtual load → run → change an input → scan → observe an output.

## 6. Build, run, and observe the simple ladder example

Use this sequence exactly. The buttons become available as each previous step is completed.

1. At the bottom of the workbench, click **Runtime & commissioning**.
2. Click **Build** in the top toolbar. Wait for the bottom status bar to say **Build current**.
3. Click **Power on**. The virtual controller should show **Stop**.
4. Click **Preview load**. Read the virtual download preview; it shows the exact candidate fingerprint and any blockers before anything changes.
5. If the preview shows no blockers, click **Commit load**.
6. Click **Go online**. In this app, this means online with the internal simulator only; it never means online with a networked PLC.
7. Click **RUN**.
8. In the **Runtime probes** table, find the `Input` row. Choose `TRUE` in the value list, then click **Set raw**.
9. Click **Scan +1**. The virtual controller executes exactly one scan.
10. Look at the `Input` and `Output` rows. Their values show the logic result. Set the input back to `FALSE`, click **Set raw**, then click **Scan +1** again to see the virtual output change back.

If a runtime button is disabled, open the **Diagnostics** tab. The app tells you which project condition is blocking the next step. Fix the named issue; do not try to bypass it.

## 7. Other features you can use now

### Program, data, and type authoring

With **Controller** selected, the **+** menu currently lets you add:

- **Organization block** — a cyclic SCL program block.
- **Ladder organization block** — the editable LAD program shown above.
- **Function** — a reusable SCL function.
- **FBD function** — a graphical Function Block Diagram function.
- **Function block** — a state-owning SCL block.
- **Global data block** and **Instance data block** — places for virtual controller data.
- **Named structure** — a named type with editable members and arrays.
- **Tag table** — a place for input, output, and memory tags.
- **Watch table** — a persistent list of virtual values to watch.
- **Trace configuration** — a controlled virtual value capture.

For an SCL block, click the block, type in the **SCL source** area, and click **Apply SCL source**. For a data block or named structure, use **Add member** or **Add array**, edit the row, then click **Apply member changes**. For an FBD block, use the displayed member lists to choose which member each function block diagram node reads or writes.

### Watch tables, traces, and virtual value changes

1. Before building, select **Controller**, click **+**, and add a **Watch table** and/or **Trace configuration**. Create tags first so these views have values to include.
2. Build, load, and go online as described above.
3. In **Runtime & commissioning**, click **Start monitoring** to update watch values during virtual scans.
4. In the **Traces** section, click **Arm**, then use **Scan +1**. The trace moves through its virtual capture states and completes after its bounded sample window.
5. In a runtime probe row, use:
   - **Set raw** — changes a virtual *input* value.
   - **Modify** — makes a one-time virtual value change.
   - **Force** — holds a virtual effective value until you click **Remove force**. Forced values are clearly marked **FORCED**.

These features apply only inside the simulator. They cannot change a real I/O point or physical device.

### Snapshots and replay

1. While online, click **STOP**.
2. Click **Capture snapshot**.
3. Change values or run more scans.
4. Click **STOP**, then click **Restore snapshot** to return the virtual controller to the saved virtual state.
5. Click **Verify replay**. A successful result shows a deterministic replay receipt with its event count and fingerprint.

### Diagnostics and virtual fault learning

The **Diagnostics** tab reports project errors and lets you navigate to the related item when possible. The runtime panel also shows runtime diagnostics. Selecting a diagnostic can take you back to the logic or object that caused it.

The simulator includes a virtual watchdog/fault learning path. If an authored program causes a virtual runtime fault, the virtual controller can enter a faulted state and show a diagnostic. This affects only the in-memory training controller. It does not affect Windows, a PLC, or any attached equipment.

## 8. Save, close, and reopen

### Save

In the approved native Windows shell:

1. Click the Save icon, or press `Ctrl+S`.
2. For the first save, use **Save as** / `Ctrl+Shift+S` and choose a name.
3. The app writes a simulator-native `.vlabproj` file and verifies the write before showing the project as saved.

The browser preview may show that local file grants are unavailable. This is expected and deliberate; it prevents the preview from becoming a general file access tool.

### Close and reopen

1. Click **Close**.
2. If you have unsaved changes, choose **Cancel**, **Discard**, or **Save and close**.
3. From the landing page, click **Choose project file** to reopen a saved `.vlabproj` project in the native shell.

When you reopen a project, its virtual controller begins powered off. Build, load, and go online again before you run it.

## 9. Simple safety boundaries and current limits

- Every item is fictional and virtual. There is no real controller connection, driver, device discovery, industrial protocol, machine download, or physical I/O access.
- **Go online** means online with the built-in virtual controller only.
- The native Windows save/open path is deliberately strict. It rejects unsafe file locations such as remote, removable, redirected, provider-backed, or special locations instead of taking a risk.
- The current preview does not provide a general-purpose file browser, a real PLC export, or a deployable project file for industrial hardware.
- Advanced ladder editing, including adding arbitrary branches for a full start/stop latch, is not yet available. Use the provided ladder template for learning and wait for a later phase for broader authoring tools.

## 10. Troubleshooting

| What you see | What it means and what to do |
| --- | --- |
| The preview link does not open. | The local preview server has stopped. Ask for it to be restarted. |
| **Create** is disabled. | Wait for the Core to finish starting and make sure the project name is not blank. |
| Build is disabled or says blocked. | Open **Diagnostics**, create a valid virtual controller configuration, and fix each blocking message. |
| **Preview load**, **Go online**, **RUN**, or **Scan +1** is disabled. | Follow the order: Build → Power on → Preview load → Commit load → Go online → RUN. |
| There are no runtime probes. | Add tags, bind them through the available program templates, build, and commit a fresh virtual load. |
| Open/save is unavailable. | This browser preview does not have the approved native project-file broker. Work in the current session, or use the native Windows shell to save/open `.vlabproj` files. |
| “Command not completed” appears. | The app safely refused an invalid action. Read the message or diagnostic, correct the project state, and try again. |
| A close dialog appears. | Use **Cancel** to continue, **Save and close** to keep a verified save, or **Discard** to lose unsaved changes. |

## For project operators

The strict native build and verification workflow is documented in [`tools/phase2/NATIVE_E2E_WORKFLOW.md`](tools/phase2/NATIVE_E2E_WORKFLOW.md). That workflow is separate from normal exploration and does not add any ability to communicate with physical hardware.
