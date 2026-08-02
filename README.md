# popnPad / [PChordPad](https://github.com/OffbeatDX/PChordPad)

Touch pad for pop'n under [spice2x](https://spice2x.github.io/).

![Demo GIF](assets/demo.gif)

## Usage

> [!CAUTION]
> In order to make the game's subscreen window fullscreen borderless, popnPad must also be run as administrator.

- Run spice2x game with **API TCP Port** as **1337** (`-api 1337`), no API Pass (`-apipass`), and **windowed** (to allow for subscreen/pad back and forth)
- Start popnPad, adjust settings as necessary
- Enjoy!

## Build

Requires [Rust](https://rustup.rs/) **1.92+**.

```
create-release.bat
```

Linux and macOS are not supported.

## Versioning

Releases use `YYYY.M.D.V` based on the UTC release date.  
`V` increments from existing tags for that date, and the version is shown in Settings.

## License

popnPad source code is licensed under the [MIT License](LICENSE).

Slint is used under the [Slint Royalty-free Desktop, Mobile, and Web Applications License](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md).

[![Made with Slint](assets/made-with-slint.png)](https://slint.dev/)
