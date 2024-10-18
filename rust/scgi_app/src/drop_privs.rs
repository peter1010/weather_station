use libc;
use std::ffi::CString;
use std::os::unix::fs::chown;
use weather_err::{Result, WeatherError};


//----------------------------------------------------------------------------------------------------------------------------------
fn get_uid_and_gid(uid_name : &str, gid_name : &str) -> Result<(u32, u32)> {
    let cstr = CString::new(uid_name.as_bytes())?;

    let p = unsafe { libc::getpwnam(cstr.as_ptr()) };
    if p.is_null() {
        return Err(WeatherError::from("Failed to get user"));
    }
    let uid = unsafe { (*p).pw_uid };

    let cstr = CString::new(gid_name.as_bytes())?;

    let p = unsafe {libc::getgrnam(cstr.as_ptr()) };
    if p.is_null() {
        return Err(WeatherError::from("Failed to get group"));
    }
    let gid = unsafe { (*p).gr_gid };

    Ok((uid, gid))
}


//----------------------------------------------------------------------------------------------------------------------------------
pub fn drop_privs(sock_name: &str, uid_name : &str, gid_name : &str) -> Result<()> {

    let (uid, gid) = get_uid_and_gid(uid_name, gid_name)?;

    chown(sock_name, Some(uid), Some(gid))?;

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
    Ok(())
}
