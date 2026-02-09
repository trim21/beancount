use beancount_parser::parse_strict;

#[test]
fn strict_error_reports_position() {
  let src = "2014-01-01 open Assets:Cash\nbroken line";

  let errors = parse_strict(src).expect_err("strict parse should fail");
  let first = &errors[0];

  assert_eq!(first.line, 2);
  assert_eq!(first.column, 1);
  assert!(
    first.message.contains("expected"),
    "strict error should include expected tokens"
  );
  assert!(first.message.contains("found 'b'"));
}

#[test]
fn strict_error_includes_summary() {
  let src = "abc";

  let errors = parse_strict(src).expect_err("strict parse should fail");
  let first = &errors[0];

  assert_eq!(first.line, 1);
  assert_eq!(first.column, 1);
  assert!(first.message.contains("expected"));
  assert!(first.message.contains("'a'"));
}
