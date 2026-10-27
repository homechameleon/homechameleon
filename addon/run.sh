#!/usr/bin/with-contenv bash
# HomeChameleon add-on. The Supervisor injects SUPERVISOR_TOKEN.
# This script does not read a user access token.
set -eu
exec /usr/bin/homechameleon-addon
