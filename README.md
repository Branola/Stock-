
This bot uses [Discord Webhooks](https://discordapp.com/developers/docs/resources/webhook) and Robinhood's Undocumented API to ping
stock prices and alert you when it changes.

- [Discord Webhooks](https://discordapp.com/developers/docs/resources/webhook`)
- [Robinhood](https://robinhood.com/`)
- [Robinhood API](https://github.com/sanko/Robinhood`)

This requires three environment variables to operate correct. I recommend making
a `run.cmd` or `run.sh` file, *not stored in version control* and using that
to kick off the bot.

Example `run.cmd`:
```bat
@echo off
SET DISCORD_ID=<ID>
SET DISCORD_TOKEN=<TOKEN>
SET STOCK_SYMBOL=AMD
cargo run
```
