Set-Location ./src-tauri/

if ($args[1] -eq "release") {
    cargo build -p lol_rec --release

    $suffix = ""
    if ($args[0] -eq "windows") {
        $suffix = "-x86_64-pc-windows-msvc"
    }
    else {
        Write-Error -Message 'unknown platform' -Category InvalidArgument
    }
    
    Copy-Item ./target/release/lol_rec.exe ./target/release/lol_rec${suffix}.exe -Force
} else {
    cargo build -p lol_rec 
}