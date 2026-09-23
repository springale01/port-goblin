# Port Goblin

A small Windows CLI that finds the processes listening on a TCP port and can terminate them.

## Usage

With Rust installed:

```powershell
cargo run -- 8080
cargo run -- 8080 --kill-mode
cargo run -- --help
```

Or install the executable locally:

```powershell
cargo install --path .
port-goblin 8080
port-goblin 8080 --kill
```

Example report:

```text
  [OCCUPIED] TCP :8080
  `-- node.exe  (PID 1234)
```

- Terminal output uses colored status badges, cyan ports, and a compact process tree. Redirected output is plain text; set `NO_COLOR=1` to disable color in terminals too.
- Accepts ports 1 through 65535. Inspects IPv4 and IPv6 TCP listeners, with one entry per process.
- `--kill-mode` (alias `--kill`) forcefully terminates every identified listener process. This stops the entire process, including work unrelated to this port; unsaved work can be lost.
- Before termination, verifies process identity and refuses system PIDs 0–4, itself, and unidentified owners. Some processes require an elevated terminal.
- Waits up to three seconds for release. If another process takes the port or a service restarts, reports failure without killing the replacement.
- `FREE` means no TCP listener was found and wildcard IPv4/IPv6 bind probes succeeded (IPv6 is skipped when unsupported). `UNAVAILABLE` means no listener was identified but a bind probe failed, for example due to a Windows port reservation.
- Read-only inspection returns exit code 0 for a completed report, including occupied/unavailable ports. Operational or termination failures return 1; invalid arguments return 2.

Windows only; requires `netstat` on PATH with the standard `TCP` / `LISTENING` output labels. UDP is outside this tool's scope. Reports are snapshots: other programs can claim or release a port immediately afterward. Identity checks reduce process-reuse risk but discovery and termination are not atomic.

## Development

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Windows integration tests create and terminate their own disposable IPv4/IPv6 listener processes. They do not target existing services.
