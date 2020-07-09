# rust-aha-cli

tui for aha in rust

uses configs from https://github.com/sbeckeriv/rust-workflow

[![asciicast](https://asciinema.org/a/vsIzIXSAK3jmxS0IzrIs3ciBx.svg)](https://asciinema.org/a/vsIzIXSAK3jmxS0IzrIs3ciBx)

## required env values

i have this stored at ~/.env but it should work on the command line

```
WORKFLOW_EMAIL=<your email> # legacy will be going away
AHA_DOMAIN=<your domain>
AHA_TOKEN=<your token>
```

## key layout

if a file is found at home_dir/.aha_cli_layout.toml it shall be read to override key bindings.

supported keys are listed in the key_layout.rs file format might look like

```
up: alt+j
right_alt: L
```

supported key words are found in app.rs
Arrow keys named up, down, left, right
none(null)
esc
\n(enter)
alt+(char)
ctrl+(char)
