Set-Location ./src-tauri/
cargo build -p lol_rec --release

$suffix = ""
if ($args[0] -eq "windows") {
    $suffix = "-x86_64-pc-windows-msvc"
}
else {
    Write-Error -Message 'unknown platform' -Category InvalidArgument
}

Move-Item ./target/release/lol_rec.exe ./target/release/lol_rec${suffix}.exe -Force