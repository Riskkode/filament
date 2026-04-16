# Filament

Filament is a keyboard-driven Terminal User Interface (TUI) for hierarchical note-taking and networked thought organisation. It uses a spatial canvas to manage nodes and their relationships.

https://github.com/user-attachments/assets/207ba708-30bd-49d7-a3be-3bf0d55a79ef

## Features

- **Hierarchical Nodes:** Organise notes in a tree structure with infinite nesting.
- **Networked Linking:** Create arbitrary links between nodes to form a networked graph.
- **Project Management:**
  - **Import:** Fuzzy search for existing projects in any local directory.
  - **Organisation:** Isolated project folders containing SQLite databases and configuration.
  - **Registry:** Manage your projects directly from the Start Menu.
- **Navigation:**
  - **Fuzzy Search:** Quickly jump to any node using the "Goto" mode.
  - **Spatial Movement:** Navigate the canvas with world-space cursor movement.
- **Persistence:**
  - SQLite-backed storage for nodes and history.
  - Project-specific and global configuration files.
- **History:** Support for Undo and Redo operations.

## Keybindings

Filament is designed to be entirely keyboard-driven. To view the full list of keybindings at any time, press `?` within the application to open the interactive Help tree.

## Roadmap

### Core Enhancements
- **Extended Notes:** Store larger notes and view them within the app.
- **File Integration:** Link local files and open them in their relevant applications from Filament.
~~- **Search Actions:** Ability to link nodes directly while in "Goto" mode.~~ completed

### Tagging System
- **Tagging:** Ability to tag any node with one of many available tags.
- **Time Tags:** Today/tomorrow tags that stamp a time onto the node.
- **Groups:** Abstract/user input tags for custom grouping.
- **Status Tags:** Integrated task management workflow:
  - `Todo` / `In Progress` / `Review` / `Completed`
  - `Pending` / `Blocked`
- **Semantic Tags:**
  - Automatic price list calculations.
  - Automatic task completion percentage calculations for parent nodes.

### Data Views
- **Tag Headers:** Dedicated lists that aggregate all nodes containing a specific tag. (will be used for project management)


### Filament Portals
- **Content linking:** External content links such as videos, images or documents viewable via configurable defaults.  
- **Filament chaining** Major feature allowing for the creation of links between externally hosted Filament instances. (collaboration)
  - Will have associated permissions, publishing and security features for the web. 

## Installation

```bash
cargo build --release
```

The binary will be located in `target/release/filament`.
