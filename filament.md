filament is a mind map app

- filament allows users to create mindmaps by drawing lines and connecting them in the TUI
- A user can write anywhere on the `screen, and connect any node to any other node.



┌────────────────────────────┬──────────────────────────────────────┐
│           Before           │                After                 │
├────────────────────────────┼──────────────────────────────────────┤
│ Normal                     │ Normal                               │
├────────────────────────────┼──────────────────────────────────────┤
│ Insert + Edit              │ Input { action, buf, cursor }        │
├────────────────────────────┼──────────────────────────────────────┤
│ Confirm                    │ dissolved                            │
├────────────────────────────┼──────────────────────────────────────┤
│ Reparent                   │ Reparent (genuinely distinct)        │
├────────────────────────────┼──────────────────────────────────────┤
│ Nodes + NodeNew + NodePick │ Canvas { cursor_x, cursor_y, state } │
└────────────────────────────┴──────────────────────────────────────┘


canvas mode has a centre selection box that represents the selection target.

Selection target is usually a node and it can be edited or changed 
depending on what command you use on that target (reparent etc)

canvas targeting will let us yank and put nodes around the screen, it will also let us quickly link nodes.

canvas mode will also let us use space as a launcher mode for various commands or menus. 
hjkl will let us move around the screen 
HJKL will use a SDF jump to hop to the closest node in the direction. 


--- 

link behaviour
- links are arrows that connect one node to another. 
- Links are represented as arrows and a line that connects the locations of the two nodes directly. 
- The arrow traverses from the start of the origin node to the start of the destination node.

# rules
The arrow should never cross the origin node.
The arrow should originate from the right side of the origin node if the destination node is to the right.
The arrow should originate from the left side of the origin noode if the destination node is to the left.
The arrow should not overlap the destination node, and should only point to the target nodes tree indicator
the arrow should always connect to the closest side of each node. 100% of the time.
originating arrows should always show when any of the containing tree nodes are selected.
arriving arrows should only show when the destination node is selected.
