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
    #[serde(rename = "timeout")]
    Timeout { name: String },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
        duration: Option<f64>,
        exec_time: Option<String>,
    },
}

fn split_name(full_name: &str) -> (&str, String) {
    let mut parts: Vec<&str> = full_name.split("::").collect();
    let name = parts.pop().unwrap_or("");
    let module_path = parts.join("::");
    return (name, module_path);
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

        if line.chars().filter(|c| !c.is_whitespace()).next() != Some('{') {
            continue;
        }

        // println!("'{}'", &line);
        let e: Event = match serde_json::from_str(&line) {
            Ok(event) => Ok(event),
            Err(orig_err) => {
                // cargo test doesn't escape backslashes to do it ourselves and retry
                let line = line.replace("\\", "\\\\");
                match serde_json::from_str(&line) {
                    Ok(event) => Ok(event),
                    Err(_) => Err(Error::new(
                        ErrorKind::Other,
                        format!("Error parsing '{}': {}", &line, orig_err),
                    )),
                }
            }
        }?;

        // println!("{:?}", e);
        match e {
            Event::Suite { event } => match event {
                SuiteEvent::Started { test_count: _ } => {
                    assert!(current_suite.is_none());
                    assert!(tests.is_empty());
                    let ts = TestSuite::new(&format!("{} #{}", suite_name_prefix, suite_index))
                        .set_timestamp(timestamp);
                    current_suite = Some(ts);
                    suite_index += 1;
                }
                SuiteEvent::Ok { results: _ } | SuiteEvent::Failed { results: _ } => {
                    assert_eq!(None, tests.iter().next());
                    r = r.add_testsuite(
                        current_suite.expect("Suite complete event found outside of suite!"),
                    );
                    current_suite = None;
                }
            },
            Event::Test {
                event,
                duration,
                exec_time,
            } => {
                let current_test_suite = current_suite
                    .as_ref()
                    .expect("Test event found outside of suite!")
                    .clone();
                let duration_ns = match (duration, exec_time) {
                    (_, Some(s)) => {
                        assert_eq!(s.chars().last(), Some('s'));
                        let seconds_chars = &(s[0..(s.len() - 1)]);
                        let seconds = seconds_chars.parse::<f64>().unwrap();
                        (seconds * 1_000_000_000.0) as i64
                    }
                    (Some(ms), None) => (ms * 1_000_000.0) as i64,
                    (None, None) => 0,
                };

                let duration = Duration::nanoseconds(duration_ns);

                match event {
                    TestEvent::Started { name } => {
                        assert!(tests.insert(name));
                    }
                    TestEvent::Ok { name } => {
                        assert!(tests.remove(&name));
                        let (name, module_path) = split_name(&name);
                        current_suite = Some(current_test_suite.add_testcase(
                            TestCase::success(&name, duration).set_classname(module_path.as_str()),
                        ));
                    }
                    TestEvent::Failed { name, stdout } => {
                        assert!(tests.remove(&name));
                        let (name, module_path) = split_name(&name);
                        current_suite = Some(
                            current_test_suite.add_testcase(
                                TestCase::failure(&name, duration, "cargo test", &stdout)
                                    .set_classname(module_path.as_str()),
                            ),
                        );
                    }
                    TestEvent::Ignored { name } => {
                        assert!(tests.remove(&name));
                    }
                    TestEvent::Timeout { name: _ } => {
                        // An informative timeout event is emitted after a test has been running for
                        // 60 seconds. The test is not stopped, but continues running and will
                        // return its result at a later point in time.
                        // This event should be safe to ignore for now, but might require further
                        // action if hard timeouts that cancel and fail the test should be specified
                        // during or before stabilization of the JSON format.
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
    use regex::Regex;
    use std::io::*;

    fn parse_bytes(bytes: &[u8]) -> Result<Report> {
        parse(BufReader::new(bytes), "cargo test", Utc::now())
    }

    fn parse_string(input: &str) -> Result<Report> {
        parse_bytes(input.as_bytes())
    }

    fn normalize(input: &str) -> String {
        let date_regex =
            Regex::new(r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})\.(\d+)\+00:00").unwrap();
        date_regex
            .replace_all(input, "TIMESTAMP")
            .replace("\r\n", "\n")
    }

    fn assert_output(report: &Report, expected: &[u8]) {
        let mut output = Vec::new();
        report.write_xml(&mut output).unwrap();
        let output = normalize(std::str::from_utf8(&output).unwrap());
        let expected = normalize(std::str::from_utf8(expected).unwrap());
        assert_eq!(output, expected);
    }

    #[test]
    fn error_on_garbage() {
        assert!(parse_string("{garbage}").is_err());
    }

    #[test]
    fn success_self() {
        let report = parse_bytes(include_bytes!("test_inputs/self.json"))
            .expect("Could not parse test input");
        let suite = &report.testsuites()[0];
        let test_cases = suite.testcases();
        assert_eq!(test_cases[0].name(), "error_on_garbage");
        assert_eq!(*test_cases[0].classname(), Some("tests".to_string()));
        assert_eq!(test_cases[0].time(), &Duration::nanoseconds(213_100));

        assert_output(&report, include_bytes!("expected_outputs/self.json.out"));
    }

    #[test]
    fn success_self_exec_time() {
        let report = parse_bytes(include_bytes!("test_inputs/self_exec_time.json"))
            .expect("Could not parse test input");
        let suite = &report.testsuites()[0];
        let test_cases = suite.testcases();
        assert_eq!(test_cases[4].name(), "az_func_regression");
        assert_eq!(*test_cases[0].classname(), Some("tests".to_string()));
        assert_eq!(test_cases[4].time(), &Duration::milliseconds(72));
        assert_output(
            &report,
            include_bytes!("expected_outputs/self_exec_time.json.out"),
        );
    }

    #[test]
    fn success_single_suite() {
        let report = parse_bytes(include_bytes!("test_inputs/success.json"))
            .expect("Could not parse test input");
        assert_output(&report, include_bytes!("expected_outputs/success.json.out"));
    }

    #[test]
    fn success_timeout() {
        let report = parse_bytes(include_bytes!("test_inputs/timeout.json"))
            .expect("Could not parse test input");

        let suite = &report.testsuites()[0];
        let test_cases = suite.testcases();
        assert_eq!(test_cases[0].name(), "long_execution_time");
        assert_eq!(*test_cases[0].classname(), Some("tests".to_string()));
        assert!(test_cases[0].is_success());
        assert_output(&report, include_bytes!("expected_outputs/timeout.json.out"));
    }

    #[test]
    fn failded_single_suite() {
        let report = parse_bytes(include_bytes!("test_inputs/failed.json"))
            .expect("Could not parse test input");
        assert_output(&report, include_bytes!("expected_outputs/failed.json.out"));
    }

    #[test]
    fn multi_suite_success() {
        let report = parse_bytes(include_bytes!("test_inputs/multi_suite_success.json"))
            .expect("Could not parse test input");
        assert_output(
            &report,
            include_bytes!("expected_outputs/multi_suite_success.json.out"),
        );
    }

    #[test]
    fn cargo_project_failure() {
        let report = parse_bytes(include_bytes!("test_inputs/cargo_failure.json"))
            .expect("Could not parse test input");
        assert_output(
            &report,
            include_bytes!("expected_outputs/cargo_failure.json.out"),
        );
    }

    #[test]
    fn az_func_regression() {
        let report = parse_bytes(include_bytes!("test_inputs/azfunc.json"))
            .expect("Could not parse test input");
        assert_output(&report, include_bytes!("expected_outputs/azfunc.json.out"));
    }
}
