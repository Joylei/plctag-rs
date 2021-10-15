cargo build --all
cargo test --all -- --test-threads=1

SET CWD=%CD%
SET DRYRUN=%~1

cd %CWD%/crates/sys && cargo publish %DRYRUN%
timeout /T 10 /NOBREAK
cd %CWD%/crates/core && cargo publish %DRYRUN%
timeout /T 10 /NOBREAK
cd %CWD%/crates/log && cargo publish %DRYRUN%
timeout /T 10 /NOBREAK
cd %CWD%/crates/derive &&  cargo publish %DRYRUN%
timeout /T 10 /NOBREAK
cd %CWD%/crates/async && cargo publish %DRYRUN%
timeout /T 10 /NOBREAK
cd %CWD% && cargo publish %DRYRUN%

echo “==ALL DONE==”