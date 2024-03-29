[![CI](https://github.com/johnterickson/cargo2junit/actions/workflows/rust.yml/badge.svg)](https://github.com/johnterickson/cargo2junit/actions/workflows/rust.yml)

# cargo2junit
Converts cargo's json output (from stdin) to JUnit XML (to stdout).

To use, first install:
```
cargo install cargo2junit
```

Then, run cargo test either with `RUSTC_BOOTSTRAP=1` or with `+beta` and convert:
```
RUSTC_BOOTSTRAP=1 cargo test -- -Z unstable-options --format json --report-time | cargo2junit > results.xml
```

Or, use tee for streaming output to console as the tests run:
```
RUSTC_BOOTSTRAP=1 cargo test -- -Z unstable-options --format json --report-time | tee results.json
cat results.json | cargo2junit > results.xml
```

Once you have your XML, publish it (e.g. for Azure Pipelines):
```
  - task: PublishTestResults@2
    inputs: 
      testResultsFormat: 'JUnit'
      testResultsFiles: 'test_results.xml'
    condition: succeededOrFailed()
```
