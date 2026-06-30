// References an openssl-sys symbol so the crate (and -lssl/-lcrypto) is actually linked.
pub fn openssl_version() -> std::os::raw::c_ulong {
    unsafe { openssl_sys::OpenSSL_version_num() }
}
