---
name: orphan-cleanup
description: >-
  Find leftover Home Assistant entities, devices, and automations that no
  longer have a source. Use when the owner asks to clean unavailable entries,
  restored entities, or references to missing entities. Propose the list,
  prefer disable over delete, and remove nothing until the owner says yes.
---

# Orphan cleanup

Holly is unofficial and is not affiliated with Home Assistant or the Open Home Foundation.

## How to work

Read the registries first. Propose a list. Disable or delete only after the owner says yes to those exact items. Look up every id at runtime. Never reuse an id from an example, and never invent a room or a device name.

## Read

From the live instance, collect:

- entities that are unavailable, restored, or have no device, and how long they have been that way if the registry says
- devices with no entities, or devices whose integration entry is gone
- automations, scripts, and dashboard cards that reference an entity id the registry does not contain
- helpers that nothing references

Match references by searching the current automation and dashboard configuration for the ids you just read. An entity that is merely off, or a device that is asleep, is not an orphan. If the state is unknown because the integration is down, send the owner to repairs instead of deleting the device.

## Propose

Show a table the owner can check: name, id from the registry, why you think it is unused, and the safest action. The safest action is almost always disable, not delete.

Call out anything that would affect a lock, an alarm panel, a garage cover, or a camera, even if it looks unused. Those stay until the owner confirms that specific row.

Do not propose deleting the entity registry, a whole integration, or a dashboard. One list, one decision.

## Act

Wait for a yes that names the rows. "Clean it all up" is not yes unless you repeated the full list and the owner agreed to that list.

Order:

1. Export or copy the automation and dashboard YAML you are about to change, and show where the copy is.
2. Disable the agreed entities or automations.
3. Remove a missing entity id from an automation or card only where the owner agreed.
4. Delete a disabled entity or device only if the owner asks for deletion of those ids.

Read the registries again. Report what changed and what you left in place. If a delete fails because the integration still owns the entity, stop and say so. Do not keep retrying.
