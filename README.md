[![Build Status](https://dev.azure.com/johnterickson/rust-lang/_apis/build/status/johnterickson.cargo2junit?branchName=master)](https://dev.azure.com/johnterickson/rust-lang/_build/latest?definitionId=32&branchName=master)

# cargo2junit
Converts cargo's json output (from stdin) to JUnit XML (to stdout).

To use, first install:
```
cargo install cargo2junit
```

Then, run cargo test and convert:
```
cargo test -- -Z unstable-options --format json | cargo2junit > results.xml
```

Or, use tee for streaming output to console as the tests run:
```
cargo test -- -Z unstable-options --format json | tee results.json
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