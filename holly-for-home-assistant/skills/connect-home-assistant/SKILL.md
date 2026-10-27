---
name: connect-home-assistant
description: >-
  Connect Holly to a Home Assistant instance through the built-in MCP Server
  integration. Use when the owner wants first-time setup, a new long-lived
  token, entity exposure for Assist, a read-only connection check, or help
  with 404, 401, 403, timeouts, SSL errors, or an empty entity list.
---

# Connect to a Home Assistant instance

Holly is unofficial and is not affiliated with Home Assistant or the Open Home Foundation.

The remote server is `${HA_URL}/api/mcp`, with header `Authorization: Bearer ${HA_TOKEN}`. The owner sets both values. `${HA_TOKEN}` is a secret.

## How to work

Read the live instance before changing it. Propose the exposure list and any other change, then wait until the owner says yes. A question or a maybe is not yes. Look up areas and entities at runtime. Do not invent names, addresses, or entity ids.

## Enable the MCP server

Ask the owner to turn on the built-in Model Context Protocol Server. Official guide: https://www.home-assistant.io/integrations/mcp_server/

Either of these reaches the same integration:

- Settings, System, AI. Enable the Model Context Protocol card if it is off. The server URL ends in `/api/mcp`.
- Settings, Devices & services, Add integration, then Model Context Protocol Server.

The integration can expose all LLM APIs and can require an administrator account. Both are on by default. Leave them as they are unless the owner asks to narrow them. Do not create a second MCP integration if one already exists. Read the existing entry first.

## Expose entities to Assist

The server only offers entities exposed to Assist. Open Settings, Voice assistants, Expose, and read the current list before editing it.

Propose a list that leaves these unexposed:

- locks
- alarm panels
- garage covers (covers whose device class is a garage)

Those stay unexposed even if a broader "expose new entities" switch is on. Unexpose any of them that are already exposed, but only after the owner says yes. Expose one of them later only when the owner names the entity and the action they want.

Cameras stay unexposed unless the owner asks. Prefer a small list: lights, climate, and sensors the owner names. Resolve every entity from the live registry. Never paste an example entity id into the instance.

## Token

Create a long-lived access token from the profile security page: https://www.home-assistant.io/docs/authentication/#long-lived-access-tokens

The owner enters it in the plugin's secure secret field for `HA_TOKEN` (Plugins, Configure, or the masked secret card). They set `HA_URL` in the same place. The URL is the instance base only, with no path and no token.

Never ask for the token in chat. Never repeat a token, write it into a file, or put it in a URL. If a token shows up in the conversation, tell the owner to revoke it and create a new one, and do not quote the value.

## Read-only check

After the owner says the variables are saved, verify with one read-only call. Listing tools, or reading the live context resource, is enough. Do not turn devices on or off, and do not lock, unlock, arm, disarm, or move a cover during the check.

Report only what came back: whether the call succeeded, which tools exist, and how many entities the context listed. If the list is empty, go to "Nothing exposed" below. Do not invent entities to fill the gap.

## Troubleshoot

Read the status code and the URL you actually called before changing settings.

**404.** The path must be exactly `/api/mcp` on the base URL, with no trailing slash. A 404 also means the integration is off, the base URL already contains a path, or the owner copied an `/api/mcp/assist` URL into `HA_URL`. `HA_URL` stays the origin only.

**401.** The request has no token, the token was revoked, or extra spaces were pasted. The header is `Authorization: Bearer` plus the secret. Create a new token in the secure field. Do not debug by printing the header.

**403.** The account cannot use the endpoint. "Require an administrator account" is on by default, so a non-admin token is rejected. Say so, and let the owner decide whether to use an administrator token or turn that option off. Do not weaken it without a yes.

**Timeouts.** The instance did not answer. Check the scheme, host, and port the owner configured, and whether this agent can reach that network. A private address fails from a cloud agent. Remote access is the owner's choice. Point them at their existing remote URL or the official remote-access docs. Do not open ports or change the firewall yourself.

**SSL.** The certificate does not match the host, the URL says `https` while the instance is `http`, or a proxy presents a different name than Settings, System, Network. The internal or external URL in that screen must match the host the agent uses. Do not tell the owner to disable certificate checks.

**Nothing exposed.** The call worked and the context is empty or missing a device the owner expected. Re-read Assist exposure. Locks, alarm panels, and garage covers are hidden on purpose. An entity that was never exposed, or an LLM API that is not the one this server publishes, will not appear. Propose the exposure change and wait for yes.
