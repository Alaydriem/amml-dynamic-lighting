namespace    = "alaydriem"
name         = "dynamic-lighting"
entry        = "alaydriem_dynamic_lighting.dll"
api_version  = 3

uses {
  feature "light" {
    event "client.tick" {
      version  = 1
      required = true
    }
    event "world.left" {
      version  = 1
      required = true
    }
    event "registry.provider_ready" {
      version  = 1
      required = true
    }
    event "registry.provider_gone" {
      version  = 1
      required = true
    }
    area "client.read" {
      version  = 1
      required = true
    }
    area "actor.read" {
      version  = 2
      required = true
    }
    area "inventory.read" {
      version  = 2
      required = true
    }
    area "lighting.terrain" {
      version  = 1
      required = true
    }
    event "mods.loaded" {
      version  = 1
      required = false
    }
    event "input.lease_suspended" {
      version  = 1
      required = false
    }
    event "input.lease_resumed" {
      version  = 1
      required = false
    }
    event "input.lease_lost" {
      version  = 1
      required = false
    }
    event "input.injected_released" {
      version  = 2
      required = false
    }
    event "overlay.screenshot_taken" {
      version  = 1
      required = false
    }
    event "overlay.screenshot_written" {
      version  = 1
      required = false
    }
    area "input.lease" {
      version  = 1
      required = false
    }
  }
  feature "entity light" {
    area "lighting.entity" {
      version  = 1
      required = true
    }
  }
  feature "vibrant colour" {
    area "lighting.vibrant" {
      version  = 1
      required = true
    }
  }
}

listing {
  title       = "Dynamic Lighting"
  summary     = "A held torch or lantern lights up the world around you."
  description = <<-EOT
    When you hold a light-giving item, the terrain around you is drawn lit, and the light follows you. Works in the normal renderer and in Vibrant Visuals. Visual only: no real light level changes.
  EOT
  authors     = ["Charles R. Portwood II"]
  license     = "BSD-3-Clause"
  source      = "https://github.com/alaydriem/amml-dynamic-lighting"
  homepage    = "https://github.com/alaydriem/amml-dynamic-lighting#readme"
  tags        = ["lighting", "visual", "client"]
  icon        = "icon.png"
}
