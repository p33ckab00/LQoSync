//! Kernel controls for the compiler child. Install them only after fork.
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

#[repr(C)]
struct Ruleset {
    writes: u64,
}
#[repr(C, packed)]
struct Beneath {
    allowed: u64,
    parent: i32,
}

pub(super) fn command(command: &mut tokio::process::Command, writable: &Path) -> io::Result<()> {
    // ABI 3 is the first ABI that can restrict file truncation.
    let abi = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<u8>(),
            0usize,
            1u32,
        )
    };
    if abi < 3 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "compiler requires Landlock ABI 3",
        ));
    }
    let writes = ((1u64 << 15) - 1) & !((1 << 0) | (1 << 2) | (1 << 3));
    let attr = Ruleset { writes };
    // SAFETY: pointers refer to initialized ABI structures of the supplied sizes.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            &attr,
            std::mem::size_of::<Ruleset>(),
            0u32,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let ruleset = unsafe { OwnedFd::from_raw_fd(fd as i32) };
    let directory = std::fs::File::options()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_NOFOLLOW)
        .open(writable)?;
    if !directory.metadata()?.is_dir() {
        return Err(io::Error::other("compiler workspace is not a directory"));
    }
    let beneath = Beneath {
        allowed: writes,
        parent: directory.as_raw_fd(),
    };
    if unsafe {
        libc::syscall(
            libc::SYS_landlock_add_rule,
            ruleset.as_raw_fd(),
            1u32,
            &beneath,
            0u32,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    #[cfg(target_arch = "x86_64")]
    let arch = 0xc000003eu32;
    #[cfg(target_arch = "aarch64")]
    let arch = 0xc00000b7u32;
    #[cfg(target_arch = "arm")]
    let arch = 0x40000028u32;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "arm")))]
    let arch = 0u32;
    if arch == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "unsupported compiler sandbox architecture",
        ));
    }
    let insn = |code, jt, jf, k| libc::sock_filter { code, jt, jf, k };
    let mut filter = vec![
        insn(0x20, 0, 0, 4),
        insn(0x15, 1, 0, arch),
        insn(0x06, 0, 0, 0x80000000),
        insn(0x20, 0, 0, 0),
        insn(0x35, 0, 1, 0x40000000),
        insn(0x06, 0, 0, 0x80000000),
    ];
    for nr in [
        libc::SYS_socket,
        libc::SYS_socketpair,
        libc::SYS_connect,
        libc::SYS_sendto,
        libc::SYS_sendmsg,
        libc::SYS_io_uring_setup,
    ] {
        filter.push(insn(0x15, 0, 1, nr as u32));
        filter.push(insn(0x06, 0, 0, 0x00050000 | libc::EACCES as u32));
    }
    filter.push(insn(0x06, 0, 0, 0x7fff0000));
    // SAFETY: the child calls only kernel operations before exec. All filter memory exists before fork.
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::syscall(libc::SYS_landlock_restrict_self, ruleset.as_raw_fd(), 0u32) != 0 {
                return Err(io::Error::last_os_error());
            }
            let program = libc::sock_fprog {
                len: filter.len() as u16,
                filter: filter.as_ptr() as *mut _,
            };
            if libc::prctl(libc::PR_SET_SECCOMP, 2, &program) != 0 {
                return Err(io::Error::last_os_error());
            }
            libc::umask(0o077);
            let limit = libc::rlimit {
                rlim_cur: 64 * 1024 * 1024,
                rlim_max: 64 * 1024 * 1024,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn child_cannot_write_outside_state_or_open_sockets() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        std::fs::create_dir(&state).unwrap();
        let outside = root.path().join("outside");
        let mut child = tokio::process::Command::new("/usr/bin/python3");
        child
            .args([
                "-I",
                "-S",
                "-B",
                "-c",
                r#"
import sys, socket
open(sys.argv[1], 'w').write('allowed')
try:
    open(sys.argv[2], 'w').write('forbidden')
    sys.exit(10)
except PermissionError: pass
try:
    socket.socket(socket.AF_UNIX)
    sys.exit(11)
except PermissionError: pass
"#,
            ])
            .arg(state.join("ok"))
            .arg(&outside);
        command(&mut child, &state).unwrap();
        assert!(child.status().await.unwrap().success());
        assert!(!outside.exists());
        assert!(state.join("ok").exists());
    }
}
