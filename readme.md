# League Record

This program automatically detects when a League of Legends game is running and records it.
Currently only supports Windows and is only tested for Windows 10 with an AMD GPU.

## Release / Build

There is a release for Windows-x64, but you can build the project on your own.
This project relies on libobs (the library Obs uses in the backrgound) to record the game.
For build prerequisites look at [libobs-recorder](https://github.com/FFFFFFFXXXXXXX/libobs-recorder)
Build with `cargo tauri build`.
Package up with `tar -cvzf LeagueRecord.tar.gz -C src-tauri libobs/data libobs/obs-plugins -C libobs *.dll *.exe -C ../target/release LeagueRecord*` (assuming that you have all your obs .dll's and the data/plugin folders in src-tauri/libobs/)
