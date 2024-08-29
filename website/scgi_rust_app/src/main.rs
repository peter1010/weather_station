use libc::{getpwnam, getgrnam, getuid};
use std::ffi::CString;
use std::fs::remove_file;
use std::os::unix::fs::chown;
use std::os::unix::net::UnixListener;
use std::io::{BufReader, BufRead, Read};

const SOCK_NAME : &str = "/run/lighttpd/scgi_app";
const SOCK_USER : &str = "http";
const SOCK_GROUP : &str = "http";

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
    remove_file(SOCK_NAME);
    
    let server = UnixListener::bind(SOCK_NAME).unwrap();
    drop_privs(SOCK_USER, SOCK_GROUP);
    return server
}


fn main() {
    let server = create_socket();

    loop {
        let (conn, addr) = server.accept().unwrap();
   
        let mut buffer = BufReader::new(conn);

        loop {
            let mut hdr_length = vec![];
            let num_bytes = buffer.read_until(b':', &mut hdr_length);

            let hdr_length : u32 = std::str::from_utf8(&hdr_length).unwrap().parse().unwrap();

            let mut hdr = Vec::<u8>::with_capacity(hdr_length as usize);
            buffer.read_exact(& mut hdr);

         /* 
            hdr_dict = {}
            tokens = hdr.split(b'\0')
            idx = 0;
            end = 2 * (len(tokens) // 2)
            while idx < end:
                name = tokens[idx]
                value = tokens[idx+1]
                hdr_dict[name] = value
                idx += 2
            print(hdr_dict)
            request = hdr_dict[b'REQUEST_URI'].decode("ascii")
         */
        }
    }
    println!("Hello, world!");
}
