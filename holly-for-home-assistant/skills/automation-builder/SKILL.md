---
name: automation-builder
description: >-
  Draft a Home Assistant automation as YAML plus a plain-language summary.
  Use when the owner wants a new automation or a change to one. Look entities
  up at runtime, dry-run the draft, and save only after a yes. Locks, alarms,
  garage covers, and cameras need a second explicit confirmation.
---

# Automation builder

Holly is unofficial and is not affiliated with Home Assistant or the Open Home Foundation.

## How to work

Read existing automations and the live entity registry before drafting. Propose YAML and a summary. Save only after the owner says yes. Look up every entity at runtime from the name the owner uses. If several match, list them and ask. Do not pick one, and do not invent an id.

## Read

Find automations that already cover the same devices or the same time. Read their triggers and actions so the draft does not fight them. Resolve each target to the entity id, domain, and device class the registry returns. Note area names only as the instance reports them.

If a needed entity is not exposed to Assist, say so. Do not expose it yourself inside this skill. For a lock, alarm panel, or garage cover, leave it unexposed unless the owner already exposed that entity on purpose.

## Draft

Write both of these before any save:

- automation YAML, with the ids you looked up
- a short summary in plain language: when it runs, what must be true, what it will do, and which mode it uses

Keep the YAML specific. Use a state trigger, a time trigger, or a numeric trigger the instance supports. Prefer `mode: single` unless the owner asked for something else. Do not add a broad event trigger that matches every entity.

Show the summary first, then the YAML. Say which existing automation you are leaving alone.

## Dry run

Before asking to save, check the draft without enabling it:

- confirm every entity id is in the registry you just read
- confirm each action is a service that domain offers
- if the instance can check configuration, run that check and report the result
- describe one walk-through with the current states: would it fire right now, and what would it change

A dry run does not call the action and does not reload automations. If the check fails, fix the draft and show it again. Do not save a draft that failed the check.

## Save

Ask for a yes to save that draft. On yes, create or update that one automation and leave it in the state the owner asked for. If they did not say to enable it, save it disabled. Read it back and show the summary that is now stored.

## Extra confirmation

If the draft locks or unlocks a lock, arms or disarms an alarm, opens or closes a garage cover, or shows or moves a camera, stop after the first yes.

Ask a second question that names the action and the entity name from the registry. Continue only if the owner confirms that action. A yes to "save the automation" does not cover the lock, alarm, garage, or camera action. If they decline, remove that action and show the revised draft.

Do not add a bypass, a hidden trigger, or a way to run the sensitive action without that second confirmation.
