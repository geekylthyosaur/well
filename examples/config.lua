local super = "Super"
local term_cmd = "alacritty"

local workspace_count = 9
local bindings = {
  [{modifiers = {super, "Shift"}, key = "Escape"}] = "Exit",
  [{modifiers = {super}, key = "Return"}] = {Spawn = term_cmd},
  [{modifiers = {super}, key = "f"}] = "ToggleFullscreen",
}

for i = 1, workspace_count do
  local key = tostring(i)
  bindings[{modifiers = {super}, key = key}] = {SwitchToWorkspace = i}
end

for i = 1, workspace_count do
  local key = tostring(i)
  bindings[{modifiers = {super, "Shift"}, key = key}] = {MoveToWorkspace = i}
end

return {
  bindings = bindings,
  workspace_count = workspace_count,
  outline = {
    color = {0.5, 0.5, 0.5},
    focused_color = {0.5, 0.5, 1.0},
    thickness = 0,
  },
}
