# Context Menu and Tool Palette

## Context Menu (`context_menu.rs`)

### Purpose
Right-click context menu for the schematic canvas. Provides quick access
to clipboard operations, item manipulation, tool switching, and view controls.

### Actions
- **Cut/Copy/Paste**: Clipboard operations (stubs for now)
- **Delete**: Delete selected item
- **Rotate 90**: Rotate selected item (stub)
- **Tool Selection**: Switch between Select, Wire, Bus, Label, No Connect, Junction
- **View**: Fit to Screen, Zoom In/Out

### Architecture
- `ContextMenuAction` enum defines all possible actions
- `draw_canvas_context_menu()` renders the menu and returns the selected action
- `handle_canvas_context_menu()` in canvas_panel.rs dispatches the action

## Tool Palette (`tool_palette.rs`)

### Purpose
Vertical tool strip on the left side of the schematic canvas.
Matches the reference UI design with icon buttons for each drawing tool.

### Architecture
- Uses `draw_tool_palette()` which returns the allocated width
- Active tool is highlighted with accent background
- Each button triggers `activate_schematic_tool_direct()` on click
