Set-Location "C:\Users\Felix\Documents\Meine Dokumente\CS Projekte\LoL\league_record\src-tauri"

cargo update
cargo build -p lol_rec --release
Copy-Item "C:\Users\Felix\Documents\Meine Dokumente\CS Projekte\LoL\league_record\src-tauri\target\release\lol_rec.exe" "C:\Users\Felix\Documents\Meine Dokumente\CS Projekte\LoL\league_record\src-tauri\lol_rec-x86_64-pc-windows-msvc.exe"
cargo build --release

sudo cp "C:\Users\Felix\Documents\Meine Dokumente\CS Projekte\LoL\league_record\src-tauri\target\release\app.exe" "C:\Program Files\LeagueRecord\LeagueRecord.exe"
sudo cp "C:\Users\Felix\Documents\Meine Dokumente\CS Projekte\LoL\league_record\src-tauri\target\release\lol_rec.exe" "C:\Program Files\LeagueRecord\lol_rec.exe"
