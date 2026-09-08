//! The `be::*` assertion macros: what each one returns, what its default
//! message says, and the evaluation guarantees callers rely on.

use erris::be;
use std::cell::Cell;

/// Counts how often the macro touched its argument.
fn bump(counter: &Cell<u32>) -> u32 {
    counter.set(counter.get() + 1);
    counter.get()
}

#[test]
fn some_moves_the_value_out() {
    let v: String = be::some!(Some("owned".to_string())).unwrap();
    assert_eq!(v, "owned");
    assert!(be::some!(Option::<u8>::None).is_err());
}

#[test]
fn some_accepts_a_temporary() {
    fn make() -> Option<String> {
        Some("x".to_string())
    }

    // Regression: the borrowing form could not outlive the temporary.
    let v = be::some!(make()).unwrap();
    assert_eq!(v, "x");
}

#[test]
fn some_ref_borrows_in_place() {
    struct Config {
        name: Option<String>,
    }
    let config = Config {
        name: Some("erris".to_string()),
    };

    let name: &String = be::some_ref!(config.name).unwrap();
    assert_eq!(name, "erris");
    // The option is still owned by `config`.
    assert!(config.name.is_some());
}

#[test]
fn none_reports_its_own_default_message() {
    assert!(be::none!(Option::<u8>::None).is_ok());
    assert_eq!(
        be::none!(Some(1)).unwrap_err().to_string(),
        "expected None: Some(1)",
    );
}

#[test]
fn ok_keeps_the_error_as_the_cause() {
    let failed: Result<(), std::io::Error> = Err(std::io::Error::other("disk gone"));
    let report = be::ok!(failed, "could not load config").unwrap_err();

    assert_eq!(report.to_string(), "could not load config");
    let chain: Vec<String> = report
        .chain()
        .map(|link| link.as_error().to_string())
        .collect();
    assert!(
        chain.iter().any(|link| link.contains("disk gone")),
        "the io error survives in the chain: {chain:?}",
    );

    let passed: Result<u8, std::io::Error> = Ok(7);
    assert_eq!(be::ok!(passed).unwrap(), 7);
}

#[test]
fn err_moves_the_error_out() {
    let failed: Result<(), &str> = Err("boom");
    assert_eq!(be::err!(failed).unwrap(), "boom");

    let passed: Result<u8, &str> = Ok(1);
    assert_eq!(
        be::err!(passed).unwrap_err().to_string(),
        "expected Err: passed",
    );
}

#[test]
fn sure_yields_unit() {
    let unit: () = be::sure!(2 * 2 == 4).unwrap();
    assert_eq!(unit, ());

    let port = 0;
    assert_eq!(
        be::sure!(port > 0).unwrap_err().to_string(),
        "expected true: port > 0",
    );
}

#[test]
fn empty_and_non_empty() {
    assert!(be::empty!("").is_ok());
    assert!(be::empty!(Vec::<u8>::new()).is_ok());
    assert_eq!(
        be::empty!("hello").unwrap_err().to_string(),
        r#"expected empty: "hello""#,
    );

    assert!(be::non_empty!("hello").is_ok());
    assert_eq!(
        be::non_empty!("").unwrap_err().to_string(),
        r#"expected non empty: """#,
    );
}

#[test]
fn zero_and_non_zero() {
    assert!(be::zero!(0_u8).is_ok());
    assert!(be::zero!(1_u8).is_err());

    assert!(be::non_zero!(42_u64).is_ok());
    let retries = 0;
    assert_eq!(
        be::non_zero!(retries).unwrap_err().to_string(),
        "expected non zero: retries",
    );
}

#[test]
fn the_checked_expression_is_evaluated_once() {
    // Regression: `zero!` and friends used to evaluate their argument twice —
    // once for the test, once to build the `Ok` value.
    let counter = Cell::new(0);
    let _ = be::non_zero!(bump(&counter));
    assert_eq!(counter.get(), 1, "non_zero!");

    let counter = Cell::new(0);
    let _ = be::zero!(bump(&counter));
    assert_eq!(counter.get(), 1, "zero!");

    let counter = Cell::new(0);
    let _ = be::empty!([bump(&counter)]);
    assert_eq!(counter.get(), 1, "empty!");

    let counter = Cell::new(0);
    let _ = be::non_empty!([bump(&counter)]);
    assert_eq!(counter.get(), 1, "non_empty!");

    let counter = Cell::new(0);
    let _ = be::len!([bump(&counter)], 1);
    assert_eq!(counter.get(), 1, "len!");

    let counter = Cell::new(0);
    let _ = be::in_range!(bump(&counter), 1..=10);
    assert_eq!(counter.get(), 1, "in_range!");

    let counter = Cell::new(0);
    let _ = be::eq!(bump(&counter), 1);
    assert_eq!(counter.get(), 1, "eq!");
}

#[test]
fn comparisons_report_both_operands() {
    assert!(be::eq!(1, 1).is_ok());
    assert_eq!(
        be::eq!(1 + 1, 3).unwrap_err().to_string(),
        "expected 1 + 1 == 3, got 2 and 3",
    );

    assert!(be::ne!(1, 2).is_ok());
    assert_eq!(
        be::ne!(2, 2).unwrap_err().to_string(),
        "expected 2 != 2, got 2 and 2",
    );

    assert!(be::lt!(1, 2).is_ok());
    assert!(be::lt!(2, 2).is_err());
    assert!(be::le!(2, 2).is_ok());
    assert!(be::le!(3, 2).is_err());
    assert!(be::gt!(3, 2).is_ok());
    assert!(be::gt!(2, 2).is_err());
    assert!(be::ge!(2, 2).is_ok());
    assert!(be::ge!(1, 2).is_err());

    assert_eq!(
        be::ge!(1, 2).unwrap_err().to_string(),
        "expected 1 >= 2, got 1 and 2",
    );
}

#[test]
fn comparisons_work_on_owned_values() {
    let left = "a".to_string();
    let right = "b".to_string();
    assert!(be::ne!(left, right).is_ok());
    // Both sides are still owned here — the macro only borrowed them.
    assert_eq!(left + &right, "ab");
}

#[test]
fn matches_a_pattern_with_a_guard() {
    let state = Some(3);
    assert!(be::matches!(state, Some(n) if *n > 2).is_ok());
    assert!(be::matches!(state, Some(n) if *n > 5).is_err());
    assert_eq!(
        be::matches!(state, None).unwrap_err().to_string(),
        "expected state to match None",
    );
}

#[test]
fn contains_len_and_range() {
    assert!(be::contains!("hello world", "world").is_ok());
    assert!(be::contains!([1, 2, 3], &2).is_ok());
    assert_eq!(
        be::contains!("hello", "bye").unwrap_err().to_string(),
        r#"expected "hello" to contain "bye""#,
    );

    assert!(be::len!("abc", 3).is_ok());
    assert_eq!(
        be::len!("abc", 5).unwrap_err().to_string(),
        r#"expected "abc" to have length 5, got 3"#,
    );

    assert!(be::in_range!(7, 1..=10).is_ok());
    assert_eq!(
        be::in_range!(42, 1..=10).unwrap_err().to_string(),
        "expected 42 in 1..=10",
    );
}

#[test]
fn every_message_form_is_accepted() {
    let owned = "owned message".to_string();

    assert_eq!(
        be::sure!(false, "literal").unwrap_err().to_string(),
        "literal",
    );
    assert_eq!(
        be::sure!(false, "formatted {}", 42)
            .unwrap_err()
            .to_string(),
        "formatted 42",
    );
    assert_eq!(
        be::sure!(false, "inline {owned}").unwrap_err().to_string(),
        "inline owned message",
    );
    assert_eq!(
        be::sure!(false, owned).unwrap_err().to_string(),
        "owned message"
    );

    // The same three forms reach every macro through one shared arm.
    assert_eq!(
        be::eq!(1, 2, "values differ: {} vs {}", 1, 2)
            .unwrap_err()
            .to_string(),
        "values differ: 1 vs 2",
    );
    assert_eq!(
        be::some!(Option::<u8>::None, "required")
            .unwrap_err()
            .to_string(),
        "required",
    );
}

#[test]
fn the_report_points_at_the_macro_call_site() {
    let line = line!() + 1;
    let report = be::sure!(false).unwrap_err();

    let location = report.location();
    assert!(
        location.file().ends_with("test_be.rs"),
        "location file: {}",
        location.file(),
    );
    assert_eq!(location.line(), line);
}

#[test]
fn composes_with_the_question_mark_operator() {
    fn check(v: Option<u32>) -> erris::Result<u32> {
        let v = be::some!(v, "value is required")?;
        be::non_zero!(v, "value must be non zero")?;
        be::in_range!(v, 1..=100)?;
        Ok(v)
    }

    assert_eq!(check(Some(42)).unwrap(), 42);
    assert_eq!(check(None).unwrap_err().to_string(), "value is required",);
    assert_eq!(
        check(Some(0)).unwrap_err().to_string(),
        "value must be non zero",
    );
    assert_eq!(
        check(Some(1000)).unwrap_err().to_string(),
        "expected v in 1..=100",
    );
}

#[test]
fn a_trailing_comma_is_accepted() {
    // rustfmt adds one when it breaks a long call across lines.
    assert!(be::sure!(false,).is_err());
    assert!(be::some!(Option::<u8>::None,).is_err());
    assert!(
        be::eq!(
            1, //
            2,
        )
        .is_err()
    );
    assert_eq!(
        be::eq!(1, 2, "message",).unwrap_err().to_string(),
        "message",
    );
    assert!(be::matches!(Some(1), None,).is_err());
}
