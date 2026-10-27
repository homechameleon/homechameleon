---
name: repairs-and-reauth
description: >-
  Read Home Assistant repairs and integrations that need to sign in again.
  Use when a repair issue is open, an integration shows reauthentication
  required, or a device stopped updating after a credential change. Explain
  the issue, then fix or dismiss it only after the owner says yes.
---

# Repairs and reauthentication

Holly is unofficial and is not affiliated with Home Assistant or the Open Home Foundation.

## How to work

Read the open issues first. Group them in a proposal. Change, dismiss, or restart something only after the owner says yes to that item. Look up integration and device names from the instance. Do not invent an account, a site, or a device.

Repairs live under Settings, System, Repairs. Official overview: https://www.home-assistant.io/integrations/repairs/

## Read

Collect, without applying fixes:

- Each repair: integration, severity, and the text the instance shows.
- Integrations flagged for reauthentication or reconfiguration.
- Whether the integration entry still exists, and whether its devices are unavailable.

Group duplicates. A repair that only repeats another issue should be described once. Quote the instance's reason in plain language. If the issue text is empty, say that, and do not fill in a cause.

## Propose

For each group, say:

- what is wrong, in the words the repair uses
- what a fix would change
- what you will not touch

Reauthentication means the owner signs in again in the integration's own dialog. Ask them to open that dialog. Do not ask for a password, an API key, or a token in chat. Do not type credentials into a file. If they paste a secret, tell them to revoke it and do not repeat it.

Some repairs are informational, such as a missing themes include or a deprecated option. Explain the choice. Do not edit `configuration.yaml` or any other file unless the owner says yes to that exact edit, and show the diff first.

## Act

After a yes, do the one agreed action:

- start the repair flow the issue offers, or
- reload that integration, or
- dismiss the issue

Dismiss only when the owner agrees the issue is already handled or does not apply. Say that a dismissal does not fix the underlying integration. If the flow asks for a secret, stop and let the owner complete that field in the UI.

Read the repairs list again afterward. Report what closed, what stayed open, and any new issue. Stop there. Do not continue into an update or a deletion unless the owner asks and the matching skill applies.
