# LeagueRecord

> LeagueRecord isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.

LeagueRecord automatically detects when a League of Legends game is running and records it. \
Currently only supports Windows.

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

![screenshot-tray](https://user-images.githubusercontent.com/37913466/258664950-89779fd8-293a-42ed-be8c-95443bde2490.png)

Right-clicking the LeagueRecord tray item opens a menu.

![screenshot-tray-menu](https://user-images.githubusercontent.com/37913466/258588802-c91c5cee-4192-4398-8582-bad709760e48.png)

1. The topmost grayed out "Recording" entry is just the recording status. While LeagueRecord is recording there is a checkmark next to the text.
2. The 'Settings' button opens the LeagueRecord settings in the windows text editor. See [Settings](#settings) for more information.
3. The 'Open' button opens a window that shows you all your recordings.
4. The 'Quit' button stops LeagueRecord completely.

Double left-clicking the LeagueRecord tray icon or clicking the 'Open' button in the tray menu opens a window that shows all your recordings.

![screenshot2](https://github.com/FFFFFFFXXXXXXX/league_record/assets/37913466/d7d13b3f-53b2-4b04-9ce0-655e57c46b8e)

There are 3 parts to the window.

1. At the top left there is a small info that shows you how much space your recordings take up as well as a box with a button to open the folder in which your recordings are stored.
2. On the left side under the info box there is a list of all you recordings. The name of each recording is the timestamp of the game.
    Clicking on a recording shows it in the right part of the window. Next to the recording name there is a button to delete the recording.
3. The right part of the window shows the currently selected recording with some information about the game at the bottom.
    The timeline of the video includes markers for the most important events that happened in the game.
    In case you don't want to see ALL events because they clutter the timeline you can enable/disable them by clicking the corresponding checkbox on the bottom right.

Closing the window doesn't stop LeagueRecord. In order to completely stop LeagueRecord you have to close it via the 'Quit' button in the tray menu.

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
It opens the settings file in the windows text editor.\
Settings get applied as soon as you save and close the text editor.
If you write an invalid setting or delete an entry it gets reset to the default value. 

|       Name        |                                               Value                                               |                 Default                 | Description                                                                                                                                                                                                                                                                                |
|:-----------------:|:-------------------------------------------------------------------------------------------------:|:---------------------------------------:| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| recordingsFolder  |                           String (only valid symbols for a foldername)                            | {System Video Folder}/league_recordings | The name of the folder in which the recordings are stored. Relative paths are appended to your default video folder.                                                                                                                                                                       |
|  filenameFormat   |                                String (with special placeholders)                                 |           %Y-%m-%d_%H-%M.mp4            | Format string for naming new recordings. Can contain [special placeholders](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) in order to make each name unique. If a new recording has the same name as an already existing recording, the old recording gets overwritten! |
|  encodingQuality  |                                  positive whole number from 0-50                                  |                   30                    | Determines the size vs. quality tradeoff for the mp4 files. Zero means best encoding quality with a big filesize. 50 means heavily compressed with a small filesize.                                                                                                                       |
| outputResolution  |                       ['480p', '720p', '1080p', '1440p', '2160p', '4320p']                        |                  1080p                  | Sets the output resolution of the recordings.                                                                                                                                                                                                                                              |
|  outputFramerate  |                               [whole number > 0, whole number > 0]                                |                   30                    | Sets the framerate of the recordings as a fraction (numerator/denominator). e.g. [30, 1] => 30fps, [30, 2] => 15fps                                                                                                                                                                        |
|    recordAudio    |                            'NONE' \| 'APPLICATION' \| 'SYSTEM' \| ALL                             |               APPLICATION               | Determines what audio gets recorded. 'NONE' records no audio. 'APPLICATION' records only LoL sounds. 'SYSTEM' records all sound output of your pc (e.g music in the background). 'ALL' records everything that 'SYSTEM' records but also your microphone input.                            |
|    markerFlags    | { 'kill', 'death', 'assist', 'turret', 'inhibitor', 'dragon', 'herald', 'baron' } : true \| false |                all true                 | Choose which events are shown by default in the timeline when playing a recording.                                                                                                                                                                                                         |
| check for updates |                                           true \| false                                           |                  true                   | Determines if on start LeagueRecord checks for new releases on GitHub                                                                                                                                                                                                                      |
|     debug log     |                                           true \| false                                           |                  false                  | If true prints logs to stdout                                                                                                                                                                                                                                                              |
|     autostart     |                                           true \| false                                           |                  false                  | If true runs LeagueRecord when you start your PC                                                                                                                                                                                                                                           |

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
This project relies on libobs-recorder (and indirectly libobs) to record the game.
For build prerequisites look at [libobs-recorder](https://github.com/FFFFFFFXXXXXXX/libobs-recorder)

> Because the libobs-recorder dependency requires the `bindeps` nightly feature and `Tauri` does currently not support using 
> nightly features the build process is a little weird.
>
> The easy way is to not create an installer via `cargo tauri build` but to manually compile and copy the files to an
> output directory.
>
> The third line in build.rs (`tauri_build::build();`) causes the build errors. 
> So comment out the third line in `build.rs` and compile the project with `cargo +nightly build -Z bindeps --release`.
>
> Now copy `./libobs/` and `./target/release/app.exe` into a seperate folder. If you want, rename `app.exe` to `LeagueRecord.exe` and you're done!
>
> Alternatively you could also run
>
> ```bash
> 7z a -tzip LeagueRecord.zip ./licenses/ ./libobs/ ./target/release/LeagueRecord.exe; # create archive
> 7z rn ./LeagueRecord.zip target/release/LeagueRecord.exe LeagueRecord.exe; # move .exe to correct position
> ```
>
> to create a .zip file with all the required files.

### Build process when cargo bindeps are stable

Build with `cargo tauri build` to create an installer.
In order to pack the compiles files into a standalone archive, run

```bash
tar -cvzf LeagueRecord.tar.gz -C ./src-tauri/ ./licenses/ ./libobs/ -C ./target/release/ ./LeagueRecord.exe
```

or

```bash
7z a -tzip LeagueRecord.zip ./licenses/ ./libobs/ ./target/release/LeagueRecord.exe; # create archive
7z rn ./LeagueRecord.zip target/release/LeagueRecord.exe LeagueRecord.exe; # move .exe to correct position
```

(assuming that you have all your obs .dll's and the data/plugin folders in src-tauri/libobs/)

## License

The libobs library is licensed under the GNU General Public License v2 (GPLv2).

The Javascript library [videojs](https://github.com/videojs/video.js) is licensed under the Apache License v2.0 and the plugin video-js markers is licensed under the MIT License.

This project (LeagueRecord) is distributed under the GNU General Public License v3 (GPLv3).

In case you have any problems, suggestions or questions feel free to open an issue. :)
