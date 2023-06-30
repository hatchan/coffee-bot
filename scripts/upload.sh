#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

scp \
    ./target/arm-unknown-linux-gnueabihf/release/coffee-bot \
    coffee-bot@${COFFEE_BOT_IP}:/home/coffee-bot/coffee-bot
