# Holly for Home Assistant

![Holly for Home Assistant](assets/logo.png)

Holly is a free, open-source plugin for Cursor and Grok Bot. It talks to your Home Assistant instance through the built-in MCP server and follows a short set of skills: connect, update, repair, clean up, and draft automations, dashboards, and themes.

Holly is unofficial. It is not affiliated with, endorsed by, or maintained by Home Assistant or the Open Home Foundation. "for Home Assistant" names the software it talks to. This project does not use the Home Assistant logo or any other mark of the Open Home Foundation.

## What you get

- A remote MCP connection to `${HA_URL}/api/mcp`, authorized with `Bearer ${HA_TOKEN}`.
- Skills that read the live instance, propose a change, and act only after you say yes.
- A HomeChameleon helper for themes and area views, installed only when you ask, from https://github.com/homechameleon/homechameleon

## Install

Add this folder as a Cursor or Grok Bot plugin (or point a marketplace entry at it). The manifest is [`.cursor-plugin/plugin.json`](.cursor-plugin/plugin.json). Skills load from [`skills/`](skills/). The server definition is [`mcp.json`](mcp.json).

Plugin layout follows the Cursor plugin reference: https://cursor.com/docs/reference/plugins

When this folder moves to its own repository, publish that repository. Relative paths inside the manifest stay valid.

## Setup variables

You set both values. The plugin does not contain them.

| Variable | Secret | What to enter |
| --- | --- | --- |
| `HA_URL` | No | Base URL of the instance, with no path and no token. |
| `HA_TOKEN` | Yes | Long-lived access token. Use the secure secret field. |

`HA_TOKEN` is marked secret in the manifest. Enter it in the plugin configuration (Plugins, then Configure) or in the client's masked secret card. Do not paste it into chat. If it appears in a chat, revoke it and create a new one.

Setup steps and the status-code checks live in the `connect-home-assistant` skill. The MCP integration guide is https://www.home-assistant.io/integrations/mcp_server/ and long-lived tokens are documented at https://www.home-assistant.io/docs/authentication/#long-lived-access-tokens

## Safety rules

Every skill follows the same rules:

1. Read the instance first. Look up entities, areas, and versions at runtime.
2. Propose the change in plain language, with the YAML or the setting that would be written.
3. Act only after you say yes. One agreed change at a time.
4. Locks, alarm panels, and garage covers stay unexposed unless you explicitly expose a named one. Automations that touch those, or a camera, need a second confirmation.
5. Tokens, passwords, and keys stay in a secure secret field. The assistant must not ask for them in chat or copy them into a file.
6. Updates start with a backup, proceed one component at a time, and stop for a health check. Rollback uses that backup.
7. Nothing in this plugin is a Home Assistant product, and nothing here speaks for the Open Home Foundation.

## Skills

| Skill | Asks before it |
| --- | --- |
| `connect-home-assistant` | Enables MCP, sets Assist exposure, checks the connection with a read-only call. |
| `safe-update` | Detects the install type, backs up, updates one component, then checks health. |
| `repairs-and-reauth` | Explains repairs and sign-in prompts, then runs one agreed fix. |
| `orphan-cleanup` | Lists unused entities and broken references, disables before it deletes. |
| `automation-builder` | Writes YAML and a plain summary, dry-runs, saves on yes. |
| `dashboard-builder` | Proposes one view from live entities and writes it on yes. |
| `homechameleon-theming` | Installs HomeChameleon on request, themes from your images, previews, then applies with backup and undo. |

Theme styles, when you ask for one, are sheer, bubbles, material, starship (Starship Console in the editor), and flat.

## Links

- MCP Server integration: https://www.home-assistant.io/integrations/mcp_server/
- Long-lived access tokens: https://www.home-assistant.io/docs/authentication/#long-lived-access-tokens
- Updates and backups: https://www.home-assistant.io/common-tasks/general/
- Repairs: https://www.home-assistant.io/integrations/repairs/
- HomeChameleon: https://github.com/homechameleon/homechameleon
- Cursor plugins: https://cursor.com/docs/reference/plugins
- License: [LICENSE](LICENSE) (MIT)

## License

Released under the MIT License. Copyright (c) 2026 Holly for Home Assistant contributors.
