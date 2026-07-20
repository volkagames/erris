use erris::*;

#[test]
fn test_error_size() {
    assert_eq!(std::mem::size_of::<Report>(), std::mem::size_of::<usize>());
}

#[test]
fn test_null_pointer_optimization() {
    assert_eq!(
        std::mem::size_of::<crate::Result<(), Report>>(),
        std::mem::size_of::<usize>()
    );
}

#[test]
fn test_drop() {
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering::SeqCst;
    use std::sync::Arc;

    #[derive(Debug)]
    struct DropError {
        has_dropped: Arc<AtomicBool>,
    }

    impl std::fmt::Display for DropError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "DropError")
        }
    }

    impl std::error::Error for DropError {}

    impl Drop for DropError {
        fn drop(&mut self) {
            let already_dropped = self.has_dropped.swap(true, SeqCst);
            assert!(!already_dropped);
        }
    }

    let has_dropped = Arc::new(AtomicBool::new(false));
    let report = report!(DropError {
        has_dropped: has_dropped.clone()
    });

    drop(report);

    assert!(has_dropped.load(SeqCst));
}
