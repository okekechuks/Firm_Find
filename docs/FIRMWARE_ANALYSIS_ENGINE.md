# Firmware analysis — pending Sprint C

No analysis worker or decompressor is added by Sprint A. No firmware programs or updater executables run. Existing Library bytes remain immutable.

Before Sprint C, independent review must select and license-check pinned parsers/tools (for example Binwalk v3, Unblob and format-specific SquashFS/MTD/UBI tools) and an isolated Linux execution boundary. WSL2 alone is not sufficient isolation. Worker acceptance requires process/time/CPU/memory/disk quotas, recursion/decompression limits, traversal/link protection, immutable input, structured bounded results and cancellation/process-tree cleanup. Missing worker, unknown/signature-protected/encrypted containers and partial extraction must report explicit limitations.

File extraction does not qualify customization, rebuilding, installation or recovery. No such success is recorded until real acquired firmware passes the independently verified stage.
