set -e
set -x

for f in test_input_generators/*;
do
    echo 
    RUSTC_BOOTSTRAP=1 cargo test --manifest-path $f/Cargo.toml -- -Z unstable-options --format json --report-time > src/test_inputs/"${f##*/}".json || true
done;
