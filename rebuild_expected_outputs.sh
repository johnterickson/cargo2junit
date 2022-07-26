set -e
set -x

for f in src/test_inputs/*.json;
do 
    if [[ "$f" == 'src/test_inputs/compile_fail.json' ]]; then continue; fi
    if [[ "$f" == 'src/test_inputs/float_time.json' ]]; then continue; fi
    if [[ "$f" == 'src/test_inputs/one_suite_no_tests.json' ]]; then continue; fi
    cargo run < "$f" > "${f%.json}.out" ||  ( echo $f | grep "fail" )
done; 

mv src/test_inputs/*.out src/expected_outputs/

TEST_STDOUT_STDERR_MAX_LEN=255 cargo run < src/test_inputs/cargo_failure.json > src/expected_outputs/cargo_failure_shortened.out