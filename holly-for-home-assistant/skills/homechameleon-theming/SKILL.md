---
name: homechameleon-theming
description: >-
  Install HomeChameleon and build a theme or a room view for a Home Assistant
  dashboard. Use when the owner asks for a theme from images, a style
  (sheer, bubbles, material, starship, or flat), a new area view, or a
  preview, backup, or undo of a theme. Read first and write only after yes.
---

# HomeChameleon theming

HomeChameleon builds dashboard themes. It is not affiliated with Home Assistant or the Open Home Foundation. Holly only installs and uses it when the owner asks.

Project and install notes: https://github.com/homechameleon/homechameleon

## How to work

Read the instance and the current theme before changing either. Propose the style, the target view, and whether a background will be stored. Generate, apply, or install only after the owner says yes. Look up areas and entities at runtime. Do not invent a room name or an entity id.

## Install

If HomeChameleon is already installed, skip install and read its version. Otherwise propose this, and wait for yes:

1. In HACS, add `https://github.com/homechameleon/homechameleon` as a custom repository, category Integration.
2. Install HomeChameleon and restart when HACS asks.
3. If `configuration.yaml` does not already merge the themes folder, show these lines and add them only after a separate yes. HomeChameleon does not edit that file itself. If the lines are missing it opens a repair instead.

```yaml
frontend:
  themes: !include_dir_merge_named themes
```

4. Add the integration under Settings, Devices & services. The sidebar page is limited to administrators.

There is also an add-on for installs that do not use HACS, documented in that repository. Propose it only when HACS is not available. Do not install both unless the owner asks.

## Theme from images

Start a theme only when the owner asks, and only from images they provide. Ask which style to use. The styles are:

- sheer
- bubbles
- material
- starship (the editor labels this Starship Console)
- flat

If they do not pick one, ask. Do not default to a style they have not named. Also ask dark, light, or auto when the editor offers that choice.

A background image is off unless they turn it on. When it is on, the theme points at a file under the instance's local files, which can be fetched by anyone who can reach the instance and guess the name. Say that before generating with a background.

Call the preview path first. Preview returns the theme and writes nothing. Show the palette roles in words and the YAML name you would write. Apply only after yes.

Apply targets one dashboard view unless the owner names more than one. Leave "set as default" off unless they explicitly ask to change the default theme. The downloadable zip remains available if they want a copy without applying.

## Room designer

When the owner wants a view for an area, read the area list and ask them to pick one. Call the room helper with that area id or the area name the instance uses. It returns a new Lovelace view as JSON and YAML. It does not write a dashboard.

The view uses built-in cards unless card collections already installed on the instance fit ordinary entities. Locks, garage covers, and alarm panels in the result still need the owner's confirmation before they stay on the view. Cameras are view only.

Preview the view in light and dark with the theme you are editing, if the panel can, before any apply. Save it as a new view only after yes. Keep the previous dashboard configuration so undo is possible. Writing the view is the dashboard-builder save path: one view, then read it back.

## Backup and undo

Before apply, confirm a snapshot or backup exists for the current themes and the target dashboard. HomeChameleon records the previous theme choices and a dashboard snapshot when it applies a view. Say where that snapshot is if the instance returns it.

Undo restores those recorded themes in one step, and only after the owner asks. If the dashboard is YAML mode, say that restoring the file is refused and show them the stored copy instead of editing the file yourself.

If apply or undo fails, stop. Report the status the instance returned. Do not retry with a broader target.
