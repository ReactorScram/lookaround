# LookAround
"I want SSH into my laptop, but I keep forgetting its IP!!"

_Has this ever happened to you?_

LookAround is a Rust program for looking up your computers' MAC and IP addresses
within a LAN. There's no central server, so it's not a look-up, it's a look-around.

```text
$ lookaround client

Found 3 peers:
11:11:11:11:11:11 = 192.168.1.101:9040 # Your laptop
22:22:22:22:22:22 = 192.168.1.102:9040 # Your desktop
33:33:33:33:33:33 = 192.168.1.103:9040 # Your home server
# Claimed MAC       IP address
```

The LookAround client uses IP multicast to find LookAround servers within the
same multicast domain.

## Installation

Use the Cargo package manager from [Rust](https://rustup.rs/) to install LookAround.

```bash
cargo install lookaround
```

To run the server as a normal user all the time, 
put this systemd unit in `~/.config/systemd/user/lookaround.service`:

```ini
[Unit]
Description=LookAround

[Service]
ExecStart=lookaround server

[Install]
WantedBy=default.target
```

Then start the service, check that it's running okay, and enable it for
auto-start:

```bash
systemctl --user start lookaround
systemctl --user status lookaround
systemctl --user enable lookaround
```

## Usage
Run the server manually: (If you haven't installed it with systemd yet)

```bash
lookaround server
```

Run a client to ping all servers in the same multi-cast domain:

```bash
lookaround client
```

## Contributing
Pull requests are welcome. This is a hobby project, so I may reject 
contributions that are too big to review.

Use the [kazupon Git commit message convention](https://github.com/kazupon/git-commit-message-convention)

## License
[AGPL-3.0](https://www.gnu.org/licenses/agpl-3.0.html)

## This Git repo
This repo's upstream is https://six-five-six-four.com/git/reactor/lookaround.
