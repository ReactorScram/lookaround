# LookAround
"I want to SSH into my laptop, but I keep forgetting its IP!!"

_Has this ever happened to you?_

```text
$ lookaround client

Found 3 peers:
11:11:11:11:11:11 = 192.168.1.101 `laptop`
22:22:22:22:22:22 = 192.168.1.102 `desktop`
33:33:33:33:33:33 = 192.168.1.103 `old-laptop`
```

LookAround is a Rust program for looking up your computers' MAC and IP addresses
within a LAN. There's no central server, so it's not a look-up, it's a look-around.

The client uses IP multicast to find servers within the
same multicast domain, similar to Avahi and Bonjour.

Systems self-identify by MAC address and nicknames. Public keys with
TOFU semantics are intended before v1.0.0.

## Installation

Use the Cargo package manager from [Rust](https://rustup.rs/) to install LookAround.

```bash
cargo install lookaround
```

To auto-start the server as a normal user. 
put this systemd unit in `~/.config/systemd/user/lookaround.service`:

```ini
[Unit]
Description=LookAround

[Service]
ExecStart=/home/user/.cargo/bin/lookaround server --nickname my-desktop

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
Run the server manually: (If you didn't configure auto-start)

```bash
lookaround server --nickname my-desktop
```

On a client computer:

```bash
# Use the `find-nick` subcommnad to find an IP...
lookaround find-nick laptop
# Prints `192.168.1.101`

# Or ping it...
ping $(lookaround find-nick laptop)

# Or SSH to it...
ssh user@$(lookaround find-nick laptop)

# Or pull a file from it
# (after starting `nc -l -p 9000 < some-file` on the laptop)
nc $(lookaround find-nick laptop) 9000

# Use the `client` subcommand to find all servers in the same multicast domain
lookaround client

# Use a longer timeout if servers need more than 500 ms to respond
lookaround client --timeout-ms 1000
```

## Contributing
Pull requests are welcome. This is a hobby project, so I may reject 
contributions that are too big to review.

Use the [kazupon Git commit message convention](https://github.com/kazupon/git-commit-message-convention)

## License
[AGPL-3.0](https://www.gnu.org/licenses/agpl-3.0.html)

## This Git repo
This repo's upstream is https://six-five-six-four.com/git/reactor/lookaround.
It's mirrored on my GitHub, https://github.com/ReactorScram/lookaround

I don't use GitHub issues, so issues are in issues.md in the repo.
