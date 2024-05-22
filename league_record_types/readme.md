# LeagueRecord Typescript definitions

This package contains Typescript definitions for the settings file in `%APPDATA%/fx.LeagueRecord/` (`Settings` type) as well as the metadata files `{videoName}.json` (`GameData` type) used for storing information about the recordings.

This package has no dependencies and is just for type hints when working with said files.

> [!IMPORTANT]  
> Version X.Y.Z of the Typescript definitions is only garuanteed to be correct for `.json` files written by the same version of LeagueRecord

## Changelog

- 1.18.0: `GameData` is now `GameMetadata` and basically all the types related to it changed. The type for the settings.json file had a new property `onlyRecordRanked` added.
- 1.17.0: Changed `EventName` dragon names from `{Type}-Dragon` to `{Type}Dragon` - so for example `Infernal-Dragon` is now `InfernalDragon`
