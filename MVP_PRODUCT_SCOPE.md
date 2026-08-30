# PLC Engineering Education Platform — MVP Scope

**Owner:** Scott

**Effective:** 2026-08-30

**Status:** Active product directive

This file records the current MVP direction. Where an older phase plan reserves
HMI, assignments, hints, grading, or teacher workflows for a later phase, this
MVP direction supersedes that reservation. The permanent clean-room, offline,
and no-physical-PLC safety boundaries remain unchanged.

## Product definition

This is a PLC Engineering Education Platform with a professional-tool-familiar
workflow. It is not a literal TIA Portal clone and must not copy Siemens visual
design, branding, assets, project formats, or proprietary behavior.

The platform contains two connected experiences over one shared project and
simulation engine:

- **Student Workspace:** the learner performs the PLC engineering workflow.
- **Teacher Portal:** the instructor creates labs, controls hints, evaluates
  behavior, reviews submissions, and gives feedback.

The center of gravity is:

> Create project → configure virtual PLC → create tags → write ladder → compile
> → run → observe → discover mistakes → debug → fix → connect an HMI → complete
> and submit the lab → teacher reviews the result.

Everything that does not strengthen that loop is secondary.

## MVP acceptance journey

MVP is achieved only when a student can independently:

1. Create, name, describe, save, reopen, rename, duplicate, and delete a project.
2. Select a brand-neutral virtual PLC and add basic digital input/output modules.
3. Use automatic or manual I/O addresses.
4. Create and manage tags with name, type, address, and description validation.
5. Build a cyclic main program with real ladder editing: rungs, series elements,
   parallel and nested branches, contacts, coils, edges, timers, counters,
   comparisons, MOVE, and basic math.
6. Compile and receive useful structural and semantic errors or warnings.
7. Fix compile failures while preserving the distinction between invalid code
   and valid-but-wrong logic.
8. Run the program on a realistic cyclic virtual PLC scan.
9. Operate a virtual trainer with momentary buttons, maintained switches,
   sensors, lamps, and output/actuator indicators.
10. Monitor energized ladder paths and values, use a watch view, and debug a
    logically incorrect program without being handed the answer.
11. Build a simple HMI screen, bind controls and indicators to PLC tags, and run
    the HMI against the same simulated PLC.
12. Open a teacher-authored assignment, use only the configured staged hints,
    satisfy teacher-defined behavior tests, and submit the project.

The teacher must be able to:

1. Create and assign an exercise from a blank project or starter template.
2. Configure hardware, starter tags, permitted instructions, and hint policy.
3. Define behavior tests as input actions plus expected PLC/HMI outcomes.
4. See student status, compile attempts, behavior-test results, hints used, and
   submission state without invasive live surveillance.
5. Open the submitted project, review it, comment, and record feedback.

## Exact MVP feature box

| Area | Required result | Current state on 2026-08-30 |
|---|---|---|
| Projects | New/open/save/rename/duplicate/delete | **Partial:** new/open/save/rename work; complete project duplication/deletion UX is unproven |
| Project tree | Devices, PLC, tags, main program, HMI | **Partial:** engineering objects exist; HMI is absent |
| PLC catalog | Small brand-neutral virtual PLC catalog | **Partial:** one EDU-21 path exists; learner selection is not a useful catalog |
| Hardware | CPU plus basic digital I/O modules | **Partial:** objects and rules exist; configuration UX is shallow |
| Addressing | Automatic and manual I/O addresses | **Partial:** automatic allocation exists; manual editing UX is missing |
| Tags | CRUD, type, address, description | **Partial:** creation/rename exist; full editing is missing |
| Tag validation | Names, types, addresses, conflicts | **Partial:** domain validation exists; learner-facing editing and feedback are incomplete |
| Main program | One cyclic ladder program | **Partial:** cyclic LAD runs; authoring begins from a fixed graph |
| Ladder editor | Full basic LAD editing | **Missing at product level:** the semantic engine supports much more than the UI exposes |
| Compiler | Structural and semantic diagnostics | **Substantially present** |
| Simulation | Real cyclic scan behavior | **Substantially present** |
| Virtual I/O | Friendly pushbuttons, switches, sensors, lamps, outputs | **Missing:** raw runtime probe controls are not a training panel |
| Live monitoring | Energized contacts, rungs, and outputs | **Partial:** values are visible; live ladder power flow is missing |
| Debugging | Watch values and inspect runtime state | **Present but engineering-heavy:** needs learner-facing presentation |
| HMI editor/runtime | Simple bound controls and indicators | **Missing** |
| Assignments and tests | Teacher-authored labs and behavior checks | **Missing** |
| Hints | Progressive, contextual, teacher-controlled | **Missing** |
| Teacher dashboard | Progress, attempts, errors, results | **Missing** |
| Submission/review | Submit, inspect, comment, feedback | **Missing** |
| Safety | No pathway to physical PLCs or industrial networks | **Architecturally present; release smoke proof remains appropriate** |

## MVP ladder instruction set

- Boolean: normally open, normally closed, coil, set, reset, rising edge, falling edge.
- Timers: TON, TOF, TP.
- Counters: CTU, CTD, CTUD.
- Compare: equal, not equal, greater, less, greater/equal, less/equal.
- Data: MOVE.
- Math: ADD, SUB, MUL, DIV.

The ladder editor must understand semantic topology. Pictures placed on a canvas
without a valid graph are not ladder authoring.

## Deliberately outside MVP

- Physical PLC communication, industrial protocols, discovery, or vendor SDKs.
- Vendor import/export or deployable industrial artifacts.
- Detailed industrial network simulation.
- SCL, FBD, GRAPH, broad FB/FC/DB authoring, UDTs, or analog configuration as
  required MVP workflows. Existing implementations may remain reusable, but
  they do not block MVP.
- PID, motion, drives, safety PLCs, SCADA, historian, rich alarm systems, trace
  oscilloscope, detailed physics/digital twins, collaborative engineering,
  Git UI, or proprietary compatibility.
- Invasive classroom surveillance or exhaustive interaction forensics.

## Product-first implementation order

1. **Motor-control vertical slice:** real LAD series/parallel editing, Start/Stop
   seal-in circuit, cyclic execution, live power display, and virtual trainer.
2. **Usable PLC setup:** small PLC catalog, digital modules, manual/automatic
   addressing, and complete tag editing/validation.
3. **MVP LAD breadth:** rung operations, nested branches, keyboard/copy/paste,
   and the required instruction set.
4. **Simple HMI:** screen builder, tag bindings, and shared-runtime execution.
5. **Educational loop:** assignments, progressive hints, behavior grading,
   submission, teacher review, and feedback.
6. **MVP polish:** navigation, accessibility, recovery, onboarding, and the full
   student/teacher acceptance journey.

## Verification policy

Testing exists to answer whether the educational product works correctly.

Day-to-day work should use:

- focused unit tests for ladder transforms and instruction/scan semantics;
- integration tests for Project → LAD → Compile → Simulation, PLC → HMI, and
  Assignment → Student work → Grading;
- a few browser journeys that exercise real student and teacher workflows.

The heavyweight native isolation and evidence machinery is release-only. It
must remain bounded and must not dictate ordinary product architecture or hold
learner-facing increments hostage.
