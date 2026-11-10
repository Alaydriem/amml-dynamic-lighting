<div align="center">

<img src="./assets/readme.png" alt="Dynamic Lighting" width="256">

# Alaydriem's Dynamic Lighting

[![Release](https://img.shields.io/github/v/release/alaydriem/amml-dynamic-lighting?include_prereleases)](https://github.com/alaydriem/amml-dynamic-lighting/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/alaydriem/amml-dynamic-lighting/build.yml?branch=master&label=build)](https://github.com/alaydriem/amml-dynamic-lighting/actions/workflows/build.yml)
[![License](https://img.shields.io/github/license/alaydriem/amml-dynamic-lighting)](./LICENSE)
![Minecraft Bedrock](https://img.shields.io/badge/Minecraft_Bedrock-1.26.63-62B47A)
[![AMML](https://img.shields.io/badge/AMML-mod-4A90D9)](https://github.com/alaydriem/amml)

</div>

Illuminate your world with carried dynamic lighting! Torches, lanterns, and blocks that emit light will now dynamically emit light relative to their block light level when in your main hand or offhand, with support for both Fancy and Vibrant Visuals Graphics

## Installation

Install it with the [AMML launcher](https://github.com/alaydriem/amml/releases/latest). The launcher also installs the AMML loader and Alaydriem's Mod Manager, which this mod needs.

## Configuration

Settings are in the mod manager panel (<kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>F10</kbd>) and saved to `mods/amml/@alaydriem/dynamic-lighting/config/config.hcl`. Changes apply at once.

| Setting | Default | Description |
| --- | --- | --- |
| `lighting.enabled` | `true` | Held light-giving items light up the terrain around you |
| `lighting.round` | `true` | On: the light fades by straight-line distance, as a round pool of light. Off: it fades by block steps, like a placed torch |
| `lighting.level_override` | `0` | `0` uses the light level of the held item. `1` to `15`: every light-giving item lights at this level. Items that give no light never light |
| `keys.toggle` | `"F3+D"` | Turns the light on or off |

## Building

Requires Windows x64, [mise](https://mise.jdx.dev), rustup and [cargo-nextest](https://nexte.st).

```powershell
mise trust
mise run preflight
mise run package
mise run test
```

## License

BSD 3-Clause. Copyright (c) 2026 Charles R. Portwood II. See [LICENSE](./LICENSE).
