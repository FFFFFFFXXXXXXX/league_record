# League Record

This program automatically detects when a League of Legends game is running and records it.
Currently only supports Windows and is only tested for Windows 10 with an AMD GPU.

## Keybindings

| Key         | Function           |
|:-----------:|:------------------:|
| Space       | Play/Pause         |
| Arrow Right | +5 seconds         |
| Arrow Left  | -5 seconds         |
| Arrow Up    | +10% volume        |
| Arrow Down  | -10% volume        |
| f           | toggle fullscreen  |
| m           | toggle mute        |
| >           | +0.25 playbackrate |
| <           | -0.25 playbackrate |
| Esc         | exit fullscreen    |

## Release / Build

**Currently [static-file-server](https://github.com/halverneus/static-file-server) is used as a replacement for the broken tauri asset protocol. This will be removed when the asset protocol is fixed**

There is a release for Windows-x64, but you can build the project on your own.
This project relies on libobs (the library Obs uses in the backrgound) to record the game.
For build prerequisites look at [libobs-recorder](https://github.com/FFFFFFFXXXXXXX/libobs-recorder)
Build with `cargo tauri build`.
Package up with `tar -cvzf LeagueRecord.tar.gz -C src-tauri libobs/data libobs/obs-plugins -C libobs *.dll *.exe -C ../target/release LeagueRecord*` (assuming that you have all your obs .dll's and the data/plugin folders in src-tauri/libobs/)

## License

The GPLv2 license of the Obs project applies to all the .dll files as well as the files in the ./data and ./obs-plugins folders in the distributed version.

The static-file-server executable is licensed under the MIT License.

This project is distributed under the GNU General Public License v2 (or any later version).
