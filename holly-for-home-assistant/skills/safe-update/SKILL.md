---
name: safe-update
description: >-
  Update a Home Assistant installation one component at a time. Use when the
  owner wants to update core, the operating system, the supervisor, or an
  add-on. Detect the install type, back up first, run a health check, and
  keep a rollback path. Never update a stack in one step.
---

# Safe update

Holly is unofficial and is not affiliated with Home Assistant or the Open Home Foundation.

## How to work

Read first, propose one update, and apply it only after the owner says yes. Finish the health check before proposing the next one. Look up versions from the running instance. Do not guess the channel, the version, or the install type.

Official update notes: https://www.home-assistant.io/common-tasks/general/

## Detect the install type

Read the installation type from the instance before choosing a procedure. Use the About screen or the system information the instance returns. Treat these as different:

- Home Assistant Operating System
- Home Assistant Container
- Home Assistant Core
- Home Assistant Supervised

If the type is missing or the reports disagree, stop and say what you could read. Do not run an Operating System update on a Container install, or the other way around.

Also read pending updates and the current versions. List them. Do not start a download while reading.

## Back up first

Propose a backup and take it only after the owner says yes. Wait until the instance reports the backup finished.

- Operating System and Supervised: Settings, System, Backups. Include the data the owner cares about. A partial backup is not a rollback for a core update.
- Container and Core: the owner follows the backup method they already use for that host. Ask which copy they have. Do not invent a host path.

Skip the update if the backup fails. Say why.

## One at a time

Propose a single pending update in this order, skipping any that are not pending: core, operating system, supervisor, then one add-on. Each item needs its own yes. Do not chain them. Do not update a second add-on because the first succeeded.

Tell the owner the name and the version you read, and what the release notes say if the instance links them. Offer to wait while they read. Breaking notes are a reason to stop, not a reason to continue quietly.

## Health check

After the instance comes back, read before announcing success:

- The core reports running, and the UI answers a read-only request.
- Settings, System, Repairs has no new issue from this update. If it does, stop and hand off to the repairs skill. Do not start the next update.
- On Operating System installs, the observer or supervisor info agrees that the new version is the one you proposed.

Compare entity and integration counts with what you read before the update. A large drop is a failed health check. Report it and stop.

## Rollback

Do not delete the backup you just made.

- Operating System: restore that backup. If the boot slot still holds the previous system, say that the owner can select it, and point at the official operating-system update page. Do not wipe the slot.
- Supervised: restore the backup. Do not reinstall the supervisor as a shortcut.
- Container: the owner redeploys the previous image tag they were running. Ask them for that tag if it was not in the reading. Do not pull `latest` again as a rollback.
- Core: restore the backup, or return to the previous environment the owner used. Do not invent a pip command.

Start a rollback only after the owner says yes. One step, then a health check again.
