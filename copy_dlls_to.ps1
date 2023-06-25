if ($args[0] -eq "all") {
    New-Item "./src-tauri/target/release/win-capture" -ItemType "Directory" -ea 0
    New-Item "./src-tauri/target/debug/win-capture" -ItemType "Directory" -ea 0
    Copy-Item "./src-tauri/libobs/*.dll" "./src-tauri/target/release/" -Force
    Copy-Item "./src-tauri/libobs/*.dll" "./src-tauri/target/debug/" -Force
}
elseif ($args[0] -eq "release") {
    New-Item "./src-tauri/target/release/win-capture" -ItemType "Directory" -ea 0
    Copy-Item "./src-tauri/libobs/*.dll" "./src-tauri/target/release/" -Force
}
elseif ($args[0] -eq "debug") {
    New-Item "./src-tauri/target/debug/win-capture" -ItemType "Directory" -ea 0
    Copy-Item "./src-tauri/libobs/*.dll" "./src-tauri/target/debug/" -Force
}
