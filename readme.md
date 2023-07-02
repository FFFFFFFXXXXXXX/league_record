# LeagueRecord

> LeagueRecord isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.

LeagueRecord automatically detects when a League of Legends game is running and records it. \
Currently only supports Windows.

![screenshot](https://user-images.githubusercontent.com/37913466/187545060-f97961f2-346d-48b7-bf1b-c453cbd86776.png)

## Keybindings

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

It is possible to adjust the settings via the settings button in the tray menu. Changed settings are require a restart to apply.

|       Name       |                                              Value                                              | Description                                                                                                                                                                                                                                                                                |
|:----------------:|:-----------------------------------------------------------------------------------------------:| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| recordingsFolder |                          String (only valid symbols for a foldername)                           | The name of the folder in which the recordings are stored                                                                                                                                                                                                                                  |
|  filenameFormat  |                               String (with special placeholders)                                | Format string for naming new recordings. Can contain [special placeholders](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) in order to make each name unique. If a new recording has the same name as an already existing recording, the old recording gets overwritten! | 
| encodingQuality  |                                positive whole number from 0-50                                  | Determines the size vs. quality tradeoff for the mp4 files. Zero means best encoding quality with a big filesize. 50 means heavily compressed with a small filesize.                                                                                                                       |
| outputResolution |                      ['480p', '720p', '1080p', '1440p', '2160p', '4320p']                       | Sets the output resolution of the recordings.                                                                                                                                                                                                                                              |
| outputFramerate  |                              [whole number > 0, whole number > 0]                               | Sets the framerate of the recordings as a fraction (numerator/denominator). <br> e.g. [30, 1] => 30fps, [30, 2] => 15fps                                                                                                                                                                   |
|   recordAudio    |                              'NONE' \| 'APPLICATION' \| 'SYSTEM'                                | Determines what audio gets recorded. 'NONE' records no audio. 'APPLICATION' records only LoL sounds. 'SYSTEM' records all sound output of your pc (e.g music in the background)                                                                                                            |
|   markerFlags    |{ 'kill', 'death', 'assist', 'turret', 'inhibitor', 'dragon', 'herald', 'baron' } : true \| false| Choose which events are shown by default in the timeline when playing a recording.                                                                                                                                                                                                         |
|check for updates |                                          true \| false                                          | Determines if on start LeagueRecord checks for new releases on GitHub                                                                                                                                                                                                                      |
|    debug log     |                                          true \| false                                          | If true prints logs to stdout                                                                                                                                                                                                         |

## Resources and Performance

LeagueRecord takes up ~70MB of your disk space.

On a system with a Ryzen 3600 CPU and RX5700 GPU these are the performance numbers measured with Windows Taskmanager.

|                             |  CPU  |   RAM  |  GPU  |
| --------------------------- |:-----:|:------:|:-----:|
| idle                        |   ~0% |   ~5MB |    0% |
| record                      |   ~3% |  ~50MB |   ~4% |
| watch recording             | ~2.5% | ~160MB | ~2.5% |

The high RAM usage when watching a recording is due to using a WebView2 Window for the UI, which basically is Chromium in disguise.
This is just a rough estimate with the default settings so you can get a sense for how much resources LeagueRecord uses.

## Release / Build

There is a release for Windows-x64, but you can build the project on your own.
This project relies on libobs-recorder (and indirectly libobs) to record the game.
For build prerequisites look at [libobs-recorder](https://github.com/FFFFFFFXXXXXXX/libobs-recorder)
Build with `cargo tauri build` to create an installer.
In order to build a standalone archive, package everything up with

```bash
tar -cvzf LeagueRecord.tar.gz -C src-tauri licenses libobs/data libobs/obs-plugins -C libobs *.dll obs-ffmpeg-mux.exe obs-amf-test.exe obs-nvenc-test.exe obs-qsv-test.exe -C ../target/release LeagueRecord.exe lol_rec.exe
```

(assuming that you have all your obs .dll's and the data/plugin folders in src-tauri/libobs/)

## License

The libobs library is licensed under the GNU General Public License v2 (GPLv2).

The Javascript library [videojs](https://github.com/videojs/video.js) is licensed under the Apache License v2.0 and the plugin video-js markers is licensed under the MIT License.

This project (LeagueRecord) is distributed under the GNU General Public License v3 (GPLv3).

In case you have any problems, suggestions or questions feel free to open an issue. :)
