# Firmware vault

Vault blobs live at `%LOCALAPPDATA%\OBTAINER\vault\sha256\<first-two-hex-digits>\<64-lowercase-hex-digest>.bin`. Staging lives in `vault\staging\<32-hex-job-id>.part` on the same volume. Original filenames are metadata only; user text never determines a blob/staging path. Existing data directories and the application identity are unchanged.

Import opens the selected source with a read lease that denies writes/deletion in Windows. Ancestors are checked for reparse points. Destination directories are held by metadata-only Windows leases to prevent replacement/renaming during operations. Vault ownership is exclusive between app instances through `vault/library.lock`; workers within that owner are bounded. The normal Windows session is required for these directory leases; restricted execution can deny profile-directory handles.

The worker streams source bytes into a unique `create_new` stage, hashes exactly the bytes written, checks approved size and source metadata, flushes/synchronizes the stage, then independently hashes staging. It persists the candidate digest before publication. `hard_link(stage, final)` publishes a complete existing file without replacing any destination; an existing destination is treated as a duplicate candidate and rehashed before reuse. Final content is opened with a read-only sharing lease, rehashed and size-checked, and held through the metadata commit. No app operation mutates a published blob. This is logical immutability, not a Windows ACL security boundary against the file owner.

These operations use Windows [file sharing/reparse-point semantics](https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-createfilea) and [hard links](https://learn.microsoft.com/en-us/windows/win32/fileio/hard-links-and-junctions). [GetDriveTypeW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdrivetypew) also rejects mapped network drives that otherwise look like local drive-letter paths. A filesystem without the required hard-link operation fails explicitly; there is no overwrite or non-atomic fallback. Free space is measured before staging, requiring the approved byte size plus a 16 MiB reserve. Errors still handle subsequent full-disk/permission failures.

SQLite cannot commit a filesystem change atomically. The consistency sequence is therefore: durable job/intent → unique stage → streamed hash and sync → staged verification → durable candidate digest → no-overwrite final publication → final blob verification/lease → one SQLite transaction committing artifact, import occurrence, integrity history, completed job and audit event. Only that last commit makes the artifact visible as a completed library entry.

| Interruption point | Recovery |
| --- | --- |
| Queued, copying, or staged | Mark owned unfinished job failed with `INTERRUPTED`; remove only its validated regular staging file; source is untouched. |
| Published but metadata uncommitted | Retain complete unreferenced blob and candidate digest/recovery explanation. Never claim success. Reimporting the source independently verifies/reuses that blob. |
| Metadata transaction fails | Transaction rolls back; job becomes failed when SQLite remains writable; retain published bytes for diagnosis/reuse. A later startup reconciles still-running work if terminal state could not be saved. |
| Cancelled near publication | No incomplete library entry is committed. A complete unreferenced blob may remain if publication already occurred; it is retained for verified reuse. |
| Corrupt existing blob | Preserve it, mark integrity failure and reject duplicate reuse; no overwrite. |
| Unknown/link/inaccessible staging item | Retain it and record recovery/manual-review information. Unknown files are never deleted. |

Startup reconciles artifact existence/size in pages of 128, without loading/hashing the whole vault or trusting an old verification label. Full integrity verification remains an explicit job. Unknown orphan blobs are preserved; Phase 4 deliberately has no general vault deletion/garbage-collection feature. Source files are never renamed, deleted, patched or executed.

Before upgrading a schema-3 database, use a WAL-aware SQLite backup. The Phase 4 run retains one at `.tools/phase-4/data-before-backup.db` and records exact preservation comparisons. Copying the main `.db` file alone while WAL is active is insufficient.
