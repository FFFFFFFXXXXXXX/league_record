# LeagueRecord

> LeagueRecord isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.

LeagueRecord automatically detects when a League of Legends game is running and records it. \
Currently only supports Windows.

Downloads are available on the [Releases](https://github.com/FFFFFFFXXXXXXX/league_record/releases) page as an installer (\*.msi file) or as a portable version (\*.zip file).

In case you have any problems, questions or suggestions feel free to open an issue or send me an email (fffffffxxxxxxxfffffffxxxxxxx@gmail.com).

## Table of Contents

- [Usage](#usage)
- [Keyboard Shortcuts](#keyboard-shortcuts)
- [Settings](#settings)
- [Resources and Performance](#resources-and-performance)
- [Release / Build](#release--build)
- [License](#license)

## Usage

<!-- ![screenshot1](https://user-images.githubusercontent.com/37913466/187545060-f97961f2-346d-48b7-bf1b-c453cbd86776.png) -->

LeagueRecord launches minimized to the windows tray to get out of your way.  
By default Windows puts the icon into the tray-icon drawer.

![screenshot-tray](https://user-images.githubusercontent.com/37913466/258664950-89779fd8-293a-42ed-be8c-95443bde2490.png)

It possible to drag the LeagueRecord icon out of the drawer (see next image).
Right-clicking the LeagueRecord tray-icon opens a menu.

![screenshot-tray-menu](https://user-images.githubusercontent.com/37913466/258588802-c91c5cee-4192-4398-8582-bad709760e48.png)

1. The topmost grayed out "Recording" entry has a checkmark next to it if your game is currently being recorded.
2. The 'Settings' button opens the LeagueRecord settings in the windows text editor. See [Settings](#settings) for more information.
3. The 'Open' button opens a window that shows you all your recordings.
4. The 'Quit' button stops LeagueRecord completely.

Double left-clicking the LeagueRecord tray icon or clicking 'right-click' -> 'Open' in the tray menu opens a window that shows all your recordings.

![screenshot2](https://github.com/FFFFFFFXXXXXXX/league_record/assets/37913466/4820528e-891f-43e2-8c1f-f2b86f885fc4)

There are 3 parts to the window.

1. In the top left corner there is an info-box where you can see how much space your recordings take up as well as a button that opens the folder in which your recordings are stored.
2. On the left side under the info-box there is a list of all you recordings. The inital name of each recording is the timestamp of the game (can be adjusted in [Settings](#settings)).
    Clicking on a recording shows it in the right part of the window. When moving your mouse over a recording there are buttons to mark a recording as a 'favorite' (see [Settings](#settings)), rename a recording and delete a recording.
3. The right part of the window shows the currently selected recording with some information about the game at the bottom.
    The timeline of the video shows colored markers for the most important events that happened in the game.
    In case you don't want to see ALL events because they clutter the timeline you can show/hide eventtypes (Kills, Deaths, Assists, ...) by clicking the corresponding checkbox on the bottom right.

Just closing the window doesn't completely stop LeagueRecord because it needs to run in the background to record your games.
In order to completely stop LeagueRecord you have to right-click the tray-icon at the bottom right of your screen and click the 'Quit' button.

> [!NOTE]
> A bunch of stuff can be customized in the [Settings](#settings). Video resolution, framerate, only record ranked games, record voice-comms or only game audio, autostart LeagueRecord when you turn on your PC, ...

> [!NOTE]
> In case LeagueRecord only records a black screen instead of the game, try running the software as Admin. That should fix the Problem!

## Keyboard Shortcuts

| Key                 | Function           |
|:-------------------:|:------------------:|
| Space               | Play/Pause         |
| Arrow Right         | +5 seconds         |
| Arrow Left          | -5 seconds         |
| Shift + Arrow Right | next event         |
| Shift + Arrow Left  | previous event     |
| Arrow Up            | +10% volume        |
| Arrow Down          | -10% volume        |
| f                   | toggle fullscreen  |
| m                   | toggle mute        |
| >                   | +0.25 playbackrate |
| <                   | -0.25 playbackrate |
| Esc                 | exit fullscreen    |

## Settings

It is possible to adjust the settings via the settings button in the tray menu.
It opens the settings file in the windows text editor.  
Settings get applied as soon as you save and close the text editor.
If you write an invalid setting or delete an entry it gets reset to the default value.

|        Name         |                                               Value                                               |                 Default                 | Description                                                                                                                                                                                                                                                                                |
|:-------------------:|:-------------------------------------------------------------------------------------------------:|:---------------------------------------:| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
|  recordingsFolder   |                         String (only characters that can be in a filename)                        | {System Video Folder}/league_recordings | The name of the folder in which the recordings are stored. Relative paths are appended to your default video folder.                                                                                                                                                                       |
|   filenameFormat    |                                String (with special placeholders)                                 |           %Y-%m-%d_%H-%M.mp4            | Format string for naming new recordings. Can contain [special placeholders](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) in order to make each name unique. If a new recording has the same name as an already existing recording, the old recording gets overwritten! |
|   encodingQuality   |                                  positive whole number from 0-50                                  |                   30                    | Determines the size vs. quality tradeoff for the mp4 files. Zero means best encoding quality with a big filesize. 50 means heavily compressed with a small filesize.                                                                                                                       |
|  outputResolution   |                    ['480p', '720p', '1080p', '1440p', '2160p', '4320p'] \| null                   |                  null                   | Sets the output resolution of the recordings to a fixed resolution. If null uses the resolution of the LoL ingame window.                                                                                                                                                                  |
|   outputFramerate   |                               [whole number > 0, whole number > 0]                                |                   30                    | Sets the framerate of the recordings as a fraction (numerator/denominator). e.g. [30, 1] => 30fps, [30, 2] => 15fps                                                                                                                                                                        |
|     recordAudio     |                            'NONE' \| 'APPLICATION' \| 'SYSTEM' \| ALL                             |               APPLICATION               | Determines what audio gets recorded. 'NONE' records no audio. 'APPLICATION' records only the games' audio. 'SYSTEM' records all sound output of your pc (e.g music in the background). 'ALL' records everything that 'SYSTEM' records but also your microphone input.                      |
|     markerFlags     | { 'kill', 'death', 'assist', 'turret', 'inhibitor', 'dragon', 'herald', 'baron' } : true \| false |                all true                 | Choose which events are shown by default in the timeline when playing a recording.                                                                                                                                                                                                         |
|   checkForUpdates   |                                           true \| false                                           |                  true                   | Determines if on start LeagueRecord checks for new releases on GitHub                                                                                                                                                                                                                      |
|      debugLog       |                                           true \| false                                           |                  false                  | If true prints logs to the console and saves it to a log file names after the current date in %APPDATA%/fx.LeagueRecord/logs/                                                                                                                                                              |
|      autostart      |                                           true \| false                                           |                  false                  | If true runs LeagueRecord when you start your PC                                                                                                                                                                                                                                           |
|   onlyRecordRanked  |                                           true \| false                                           |                  false                  | If true only records Solo/DuoQ and FlexQ games                                                                                                                                                                                                                                             |
| maxRecordingAgeDays |                                     positive numbers \| null                                      |                   null                  | Recordings that are not marked as favorites (golden star) get deleted after X days. null means disabled.                                                                                                                                                                                   |
| maxRecordingsSizeGb |                                     positive numbers \| null                                      |                   null                  | Recordings that are not marked as favorites (golden star) get deleted if the size of all your recordings exceeds this number (in Gigabytes). null means disabled.                                                                                                                          |

## Resources and Performance

LeagueRecord takes up ~70MB of your disk space with most of that coming from the libobs dependency.

On a system with a Ryzen 3600 CPU and RX5700 GPU these are the performance numbers measured with Windows Taskmanager.

|                             |  CPU  |   RAM  |  GPU  |
| --------------------------- |:-----:|:------:|:-----:|
| idle                        |   ~0% |   ~5MB |    0% |
| record                      |   ~3% |  ~50MB |   ~4% |
| watch recording             | ~2.5% | ~160MB | ~2.5% |

The high RAM usage when watching a recording is due to using a WebView2 Window for the UI, which basically is a Chromium version that is pre-installed on most windows PCs. UIs are easy to make with HTML + some CSS. It also keeps the program size small and the common case - running hidden in the taskbar - efficient.

This is just a rough estimate with the default settings so you can get a sense for how much resources LeagueRecord uses.

## Release / Build

There is a release for Windows-x64, but you can build the project on your own.

In order to build the project you need to have the [nightly](https://rust-lang.github.io/rustup/concepts/channels.html#working-with-nightly-rust) Rust toolchain and [NPM](https://nodejs.org/en) installed.  
From the root folder of the project run `npm install` and then `npx tauri dev` to run the project in debug mode or `npx tauri build` to build in release-mode and create the installer.

In order to package the compiled files into a standalone archive, run one of the following commands after a successful `npx tauri build`:

```bash
# outputs 'LeagueRecord.tar.gz' in the project root folder
tar -czv -f LeagueRecord.tar.gz -C ./src-tauri/target/release/ LeagueRecord.exe libobs licenses
```

or

```bash
# outputs 'LeagueRecord.zip' in the project root folder
cd src-tauri/target/release && 7z a -tzip ../../../LeagueRecord.zip LeagueRecord.exe libobs licenses && cd ../../..
```

## License

This project (LeagueRecord) is distributed under the GNU General Public License v3 (or any later version).  
I want to release this project under an open-source license but that license needs to comply with the licenses of all my dependencies. GPLv3+ seems to be the [way to do that](https://stackoverflow.com/a/1978524).

- [`obs-studio/libobs`](https://github.com/obsproject/obs-studio) is licensed under the GNU General Public License v2 (GPLv2).
- [`videojs`](https://github.com/videojs/video.js) is licensed under the Apache License v2.0
- [`videojs-markers`](https://github.com/FFFFFFFXXXXXXX/videojs-markers) is licensed under the MIT License.
