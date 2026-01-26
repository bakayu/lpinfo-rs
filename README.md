# lpinfo-rs
Experimental Rust port of CUPS `lpinfo` using `cups_rs` bindings.

## Build
```
cargo build
```

## Usage
```
lpinfo-rs -v        # list devices
lpinfo-rs -v -l     # list devices (long)
lpinfo-rs -m        # list models/PPDs
lpinfo-rs -m -l     # list models (long)
```

## Features

All `lpinfo` flags supported.

- `-v` / `-l`: device discovery output
- `-m` / `-l`: PPD/model listing
- Filters: `--device-id`, `--language`, `--make-and-model`, `--product`
- Scheme filters: `--include-schemes`, `--exclude-schemes`
- `--timeout` for device discovery
- `-E` encryption, `-h` server/host

## Notes
- Uses direct IPP requests (CUPS-GET-DEVICES / CUPS-GET-PPDS).
- Output order may differ from `lpinfo`.
