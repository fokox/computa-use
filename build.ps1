$rustupDir = "C:\Users\Zul\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin"
$llvmDlltool = (Get-ChildItem -Path C:\Users\Zul\.rustup -Filter "llvm-dlltool.exe" -Recurse | Select-Object -First 1).FullName
New-Item -ItemType Directory -Force temp_bin | Out-Null
Copy-Item $llvmDlltool -Destination temp_bin\dlltool.exe
$env:PATH = "$(Get-Location)\temp_bin;" + $env:PATH
cargo build
cargo run
