use lockscreen::lock::ScreenLocker;
use std::cell::RefCell;
use std::path::PathBuf;

// Noop locker will not spawn subprocess
// Test without locking the screen
struct NoopLocker;
impl ScreenLocker for NoopLocker {
    fn lock(&self, _path: &PathBuf) -> Result<(), std::io::Error> {
        Ok(())
    }
}

struct CapturingLocker {
    called_with: RefCell<Option<PathBuf>>,
}

impl ScreenLocker for CapturingLocker {
    fn lock(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        *self.called_with.borrow_mut() = Some(path.clone());
        Ok(())
    }
}

#[test]
fn noop_locker_succeeds() {
    let locker = NoopLocker;
    assert!(locker.lock(&PathBuf::from("/dev/shm/test.png")).is_ok());
}

#[test]
fn capturing_locker_records_path() {
    let locker = CapturingLocker {
        called_with: RefCell::new(None),
    };
    locker.lock(&PathBuf::from("/dev/shm/test.png")).unwrap();

    assert_eq!(
        *locker.called_with.borrow(),
        Some(PathBuf::from("/dev/shm/test.png"))
    );
}
