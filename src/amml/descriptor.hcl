namespace    = "alaydriem"
name         = "dynamic-lighting"
display_name = "Dynamic Lighting"
version      = "26.62.0"
author       = "Alaydriem"
config_path  = "config.hcl"

claim "WorldDraw" {
  mode   = "Shared"
  reason = "Draws terrain near the player lit by the held light"
}

keybind "toggle" {
  label       = "Toggle dynamic lighting"
  default     = "F3+D"
  capability  = "WorldDraw"
  scope       = "Global"
  config_path = "keys.toggle"
}

section "lighting" {
  title = "Lighting"

  field "lighting.enabled" {
    label   = "Enabled"
    help    = "Held light-emitting items light up the terrain around you"
    kind    = "bool"
    default = true
    apply   = "Live"
  }

  field "lighting.round" {
    label   = "Round falloff"
    help    = "On: the light fades by straight-line distance. Off: by block steps, like a placed torch"
    kind    = "bool"
    default = true
    apply   = "Live"
  }

  field "lighting.level_override" {
    label   = "Light level override"
    help    = "0 uses the level of the held item. 1 to 15: held light items light at this level. Other items never give light"
    kind    = { int = { min = 0, max = 15, step = 1 } }
    default = 0
    apply   = "Live"
  }
}

section "keys" {
  title = "Keys"

  field "keys.toggle" {
    label   = "Toggle dynamic lighting"
    help    = "Turns the held light on or off"
    kind    = "chord"
    default = "F3+D"
    apply   = "Live"
  }
}
