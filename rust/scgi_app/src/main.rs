use libc::{getpwnam, getgrnam, getuid};
use std::ffi::CString;
use std::fs::remove_file;
use std::os::unix::fs::chown;
use std::os::unix::net::UnixListener;
use std::io::{BufReader, BufRead, Read, Write};
use std::collections::HashMap;

const SOCK_NAME : &str = "/run/lighttpd/scgi_app";
//const SOCK_USER : &str = "http";
const SOCK_USER : &str = "lighttpd";
//const SOCK_GROUP : &str = "http";
const SOCK_GROUP : &str = "lighttpd";

fn get_uid_and_gid(uid_name : &str, gid_name : &str) -> Option<(u32, u32)> {
    let cstr = CString::new(uid_name.as_bytes()).ok()?;

    let p = unsafe { libc::getpwnam(cstr.as_ptr()) };
    if p.is_null() {
        return None;
    }
    let uid = unsafe { (*p).pw_uid };

    let cstr = CString::new(gid_name.as_bytes()).ok()?;

    let p = unsafe {libc::getgrnam(cstr.as_ptr()) };
    if p.is_null() {
        return None;
    }
    let gid = unsafe { (*p).gr_gid };

    Some((uid, gid))
}

fn drop_privs(uid_name : &str, gid_name : &str) {

    let (uid, gid) = get_uid_and_gid(uid_name, gid_name).unwrap();

    chown(SOCK_NAME, Some(uid), Some(gid)).expect("Chown failed");

    let p_uid = unsafe { libc::getuid() };
    if p_uid == 0 {
        // Remove group privileges
        unsafe { libc::setgroups(0, std::ptr::null()) };

        // Try setting the new uid/gid
        unsafe { libc::setgid(gid) };
        unsafe { libc::setuid(uid) };

        // Ensure a very conservative umask
        unsafe { libc::umask(0o077) }; 
    }
 
}

fn create_socket() -> UnixListener {
    let _ = remove_file(SOCK_NAME);

    let server = UnixListener::bind(SOCK_NAME).unwrap();
    drop_privs(SOCK_USER, SOCK_GROUP);
    return server
}


fn main() {
    let server = create_socket();

    loop {
        let (conn, _addr) = server.accept().unwrap();

        let mut reader = BufReader::new(conn);

        let mut hdr_fields = HashMap::new();

        let mut hdr_length = vec![];
        let _ = reader.read_until(b':', &mut hdr_length);

        // Drop the colon
        hdr_length.pop();

        let hdr_length : u32 = std::str::from_utf8(&hdr_length).unwrap().parse().unwrap();

        let mut hdr = vec![0; hdr_length as usize];
        let _ = reader.read_exact(& mut hdr);

        let iter = hdr.split(|x| *x == b'\0');
        let mut name = String::new();
        let mut idx = 0;
        for part in iter {
            if idx == 0 {
                name = std::str::from_utf8(&part).unwrap().to_string();
                idx = 1;
            } else {
                let value = std::str::from_utf8(&part).unwrap().to_string();
                idx = 0;
                println!("{} => {}", name, value);
                hdr_fields.insert(name.clone(), value);
            }
        }
        let mut writer = reader.into_inner();
        writer.write_all(b"Status: 200 OK\r\n");
        writer.write_all(b"Content-Type: text/plain\r\n");
        writer.write_all(b"\r\n");
        writer.write_all(b"Hello, world!\r\n");
        println!("Done");
    }
}
