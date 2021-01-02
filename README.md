# Github Release Watcher

Simple bot to poll information of Github repositories every `x` minutes and send an e-mail, once a new release has been detected. I have written this small bot, as `Github Watch` did not differentiate between actual releases and (daily) pre-releases.
Therefore it will watch [/releases/latest](https://github.com/neovim/neovim/releases/latest) which links to actual releases.

## Installation

This bot is self-hosted. I do not provide a service to check for your releases, but you also don't have to trust me in return. It is therefore necessary to have a machine running 24/7. If you have a non x86_64 machine such as the RPI Zero W with ARMv6 chip set and struggle to cross-compile the source code, you can check [this Dockerfile](https://github.com/tfachmann/docker-pipelines/tree/master/rust/rpi_armv6).

## Usage

Sending an e-mail appears to be non-trivial, so I expect you already configured a mail client to send mails (e.g. `smtp` or `msmtp`) and the command `mail` is available. Therefore this is also limited to linux so far.

## Configuration

Create `~/.config/gh-release-watcher/config.toml` and configure it as

```toml
[config]
email = "foo@bar.com"   # who to send the email to
time = 3600             # polling time (in sec)

[github]
"neovim/neovim" = "0"   # will be updated by the bot, if a newer version has been found
```

If changes to the configuration are made, the program has to be restarted.
