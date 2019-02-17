extern crate junit_report;
extern crate serde;

use junit_report::*;
use serde::{Deserialize, Serialize};
use std;
use std::collections::BTreeSet;
use std::io::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct SuiteResults {
    passed: usize,
    failed: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "event")]
enum SuiteEvent {
    #[serde(rename = "started")]
    Started { test_count: usize },
    #[serde(rename = "ok")]
    Ok {
        #[serde(flatten)]
        results: SuiteResults,
    },
    #[serde(rename = "failed")]
    Failed {
        #[serde(flatten)]
        results: SuiteResults,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "event")]
enum TestEvent {
    #[serde(rename = "started")]
    Started { name: String },
    #[serde(rename = "ok")]
    Ok { name: String },
    #[serde(rename = "failed")]
    Failed { name: String, stdout: String },
    #[serde(rename = "ignored")]
    Ignored { name: String },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
enum Event {
    #[serde(rename = "suite")]
    Suite {
        #[serde(flatten)]
        event: SuiteEvent,
    },
    #[serde(rename = "test")]
    Test {
        #[serde(flatten)]
        event: TestEvent,
    },
}

fn parse<T: BufRead>(
    input: T,
    suite_name_prefix: &str,
    timestamp: DateTime<Utc>,
) -> Result<Report> {
    let mut r = Report::new();
    let mut suite_index = 0;
    let mut current_suite: Option<TestSuite> = None;
    let mut tests: BTreeSet<String> = BTreeSet::new();

    for line in input.lines() {
        let line = line?;

        if line.chars().next() != Some('{') {
            continue;
        }

        // println!("'{}'", &line);
        let e: Event = serde_json::from_str(&line).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Error parsing '{}': {}", &line, e),
            )
        })?;
        // println!("{:?}", e);
        match e {
            Event::Suite { event } => match event {
                SuiteEvent::Started { test_count: _ } => {
                    assert!(current_suite.is_none());
                    assert!(tests.is_empty());
                    let mut ts = TestSuite::new(&format!("{} #{}", suite_name_prefix, suite_index));
                    ts.set_timestamp(timestamp);
                    current_suite = Some(ts);
                    suite_index += 1;
                }
                SuiteEvent::Ok { results: _ } | SuiteEvent::Failed { results: _ } => {
                    assert!(tests.is_empty());
                    r.add_testsuite(
                        current_suite.expect("Suite complete event found outside of suite!"),
                    );
                    current_suite = None;
                }
            },
            Event::Test { event } => {
                let current_suite = current_suite
                    .as_mut()
                    .expect("Test event found outside of suite!");
                match event {
                    TestEvent::Started { name } => {
                        assert!(tests.insert(name));
                    }
                    TestEvent::Ok { name } => {
                        assert!(tests.remove(&name));
                        current_suite.add_testcase(TestCase::success(&name, Duration::zero()));
                    }
                    TestEvent::Failed { name, stdout } => {
                        assert!(tests.remove(&name));
                        current_suite.add_testcase(TestCase::failure(
                            &name,
                            Duration::zero(),
                            "cargo test",
                            &stdout,
                        ));
                    }
                    TestEvent::Ignored { name } => {
                        assert!(tests.remove(&name));
                    }
                }
            }
        }
    }

    Ok(r)
}

fn main() -> Result<()> {
    let timestamp = Utc::now();
    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    let report = parse(stdin, "cargo test", timestamp)?;

    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    report
        .write_xml(stdout)
        .map_err(|e| Error::new(ErrorKind::Other, format!("{}", e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::parse;
    use junit_report::*;
    use std::io::*;

    fn parse_bytes(bytes: &[u8]) -> Result<Report> {
        parse(BufReader::new(bytes), "test", Utc::now())
    }

    fn parse_string(input: &str) -> Result<Report> {
        parse_bytes(input.as_bytes())
    }

    #[test]
    fn error_on_garbage() {
        assert!(parse_string("{garbage}").is_err());
    }

    #[test]
    fn success_single_suite() {
        let _report = parse_bytes(include_bytes!("test_inputs/success.json"))
            .expect("Could not parse test input");
    }

    #[test]
    fn failded_single_suite() {
        let _report = parse_bytes(include_bytes!("test_inputs/failed.json"))
            .expect("Could not parse test input");
    }

    #[test]
    fn multi_suite_success() {
        let _report = parse_bytes(include_bytes!("test_inputs/multi_suite_success.json"))
            .expect("Could not parse test input");
    }

    #[test]
    fn cargo_project_failure() {
        let _report = parse_bytes(include_bytes!("test_inputs/cargo_failure.json"))
            .expect("Could not parse test input");
    }
}
