# This script is only intended for use during development. It's faster than a
# regular `cargo install`, which makes iteration more pleasant.

$cargoHome = if ($env:CARGO_HOME) {
    $env:CARGO_HOME
} else {
    "$HOME/.cargo"
}
$cargoTargetDir = if ($env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR
} elseif ($env:CARGO_BUILD_TARGET_DIR) {
    $env:CARGO_BUILD_TARGET_DIR
} else {
    "target"
}

function copyy {
    cp "$cargoTargetDir\debug\cargo-$args.exe" "$cargoHome\bin\cargo-$args.exe"
}

cargo build -p cargo-mobile2
copyy "android"
copyy "apple"
copyy "mobile"

