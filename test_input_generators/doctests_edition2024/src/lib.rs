/// Pass:
/// ```
/// assert_eq!(1, 1)
/// ```
///
/// Fail:
/// ```
/// assert_eq!(1, 2)
/// ```
///
/// ```should_panic
/// assert_eq!(2, 1)
/// ```
///
/// ```ignore
/// assert_eq!(2, 1)
/// ```
pub fn foo() {}
