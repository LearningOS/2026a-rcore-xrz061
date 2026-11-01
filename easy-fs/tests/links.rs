use easy_fs::{BlockDevice, EasyFileSystem};
use std::sync::{Arc, Mutex};

struct MemoryDisk(Mutex<Vec<[u8; 512]>>);

impl BlockDevice for MemoryDisk {
    fn read_block(&self, id: usize, buffer: &mut [u8]) {
        buffer.copy_from_slice(&self.0.lock().unwrap()[id]);
    }

    fn write_block(&self, id: usize, buffer: &[u8]) {
        self.0.lock().unwrap()[id].copy_from_slice(buffer);
    }
}

#[test]
fn links_open_unlink_and_reclamation() {
    let disk = Arc::new(MemoryDisk(Mutex::new(vec![[0; 512]; 4096])));
    let fs = EasyFileSystem::create(disk, 4096, 1);
    let root = EasyFileSystem::root_inode(&fs);
    let file = root.create("original").unwrap();
    let id = file.stat().0;
    file.write_at(0, b"shared contents");
    assert!(!root.link("missing", "alias"));
    assert!(!root.link("original", "original"));
    assert!(root.link("original", "alias"));
    assert!(!root.link("original", "alias"));
    let alias = root.find("alias").unwrap();
    assert!(Arc::ptr_eq(&file, &alias));
    assert_eq!(alias.stat(), (id, false, 2));
    assert!(root.unlink("original"));
    assert!(root.unlink("alias"));
    assert!(!root.unlink("alias"));
    assert!(root.find("alias").is_none());
    assert_eq!(file.stat().2, 0);
    let mut buffer = [0; 15];
    assert_eq!(alias.read_at(0, &mut buffer), 15);
    assert_eq!(&buffer, b"shared contents");
    drop(file);
    drop(alias);
    let data = [42; 16384];
    // More allocations than this disk can hold without recycling inode/data bits.
    for _ in 0..300 {
        let file = root.create("reused").unwrap();
        assert_eq!(file.stat().0, id);
        assert_eq!(file.write_at(0, &data), data.len());
        drop(file);
        assert!(root.unlink("reused"));
    }
    assert!(root.ls().is_empty());
    assert!(root.create(&"x".repeat(28)).is_none());
}
