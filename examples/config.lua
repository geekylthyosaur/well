local super = "Super"
local term_cmd = "alacritty"

return {
  bindings = {
    [{modifiers = {super, "Shift"}, key = "Escape"}] = "Exit",
    [{modifiers = {super}, key = "Return"}] = {Spawn = term_cmd},
  },
}
