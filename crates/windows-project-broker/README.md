# Windows project-file broker

This crate is the only approved native host-file boundary for the Windows-first
Phase 2 configuration. It is governed by `P2-DEC-ISO-NATIVE-001` and ADR-0005.

The executable accepts a closed, versioned binary protocol over inherited
anonymous standard handles. It accepts only project-list, project-open,
project-save-as, project-save, revoke, and handshake frames. A request can name
one bounded `.vlabproj` base name; it can never supply a path, URI, endpoint,
native method, shell command, or generic operation. The executable accepts no
command-line arguments and never starts another process or opens a network,
device, industrial-communication, print, or deployable-export capability.

The fixed project root is derived from the authoritative Windows
`FOLDERID_LocalAppData` known folder and appends only `GovsPLC\Projects`;
caller-controlled environment values never select it. Startup and every file
transaction fail closed unless the Windows backing attestor proves:

- an absolute drive-letter path on the native Windows system volume and an
  NTFS or ReFS filesystem;
- a reviewed native storage bus with no removable-media or device/media-hotplug
  indication (USB, iSCSI, virtual/file-backed, SD/MMC, 1394, Spaces, Fibre, and
  unknown bus types are rejected even if Windows reports `DRIVE_FIXED`);
- no reparse, offline, recall-on-open, recall-on-data-access, device, or
  provider-backed attribute on any admitted component;
- no remote protocol on the opened handle;
- a normalized final DOS path inside the attested root; and
- a one-link regular file with stable volume/file, last-write, size, and
  selected-content identity.

Enumeration is metadata-only. The selected project is content-bound only after
the host-owned chooser accepts it. Save As is create-only; Save writes a
verified temporary file and uses conditional atomic replacement with rollback
and displaced-file re-attestation if concurrent identity changes are observed.

The browser File System Access API is not an accepted production path. The web
workbench can reach this broker only through the exact native bridge contract
in `apps/foundation-shell/src/file-access-broker.ts`; ordinary browser pickers
are deliberately ignored and fail closed.
