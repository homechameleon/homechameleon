---
name: dashboard-builder
description: >-
  Build or edit a Home Assistant dashboard view from the live entity list.
  Use when the owner wants a new view, a card change, or a simpler layout.
  Read the current dashboard, propose YAML and a plain summary, and write
  only after the owner says yes.
---

# Dashboard builder

Holly is unofficial and is not affiliated with Home Assistant or the Open Home Foundation.

## How to work

Read the current dashboards before proposing a layout. Write only after the owner says yes. Look up entities, areas, and the dashboard storage mode at runtime. Do not invent rooms, people, or entity ids. If the owner describes a place, match it to an area the instance returns, or ask.

This skill stores Lovelace views. A themed room view that comes from HomeChameleon belongs to the homechameleon-theming skill. Do that work there, then return here if the owner wants ordinary cards.

## Read

List dashboards and whether each is storage-mode or YAML-mode. Read the view titles, card types, and entity ids already on the view you would edit. Note installed cards only if the instance shows them. Prefer built-in cards when nothing else is installed.

A YAML-mode dashboard is a file the owner maintains. Do not rewrite that file unless they say yes to a specific patch, and show the patch first. Storage-mode dashboards can be updated through the dashboard API after a yes.

## Propose

Offer one new view or one edit to an existing view. Include:

- a plain summary of the sections and which area each section uses
- the view YAML, using entity ids from the reading
- what you will not change, including other views and the default dashboard

Keep locks, alarm panels, and garage covers off the view unless the owner asked for that entity. Cameras, if requested, are view-only: no controls that move or enable them. Say that in the summary.

Match density to the request. A few sections beat a card for every entity in the area. Leave hidden and unexposed entities out.

## Preview and save

Describe the view as the owner will see it: section titles, card types, and the entity names. This description is the preview. Do not save in order to preview.

On yes, keep a copy of the previous dashboard configuration, then write the one agreed view. Read the dashboard back and confirm the view title and the entity ids. If the write fails, restore the copy and say so.

Undo is that copy, applied only after the owner asks. Do not delete other views as cleanup.
