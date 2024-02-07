use std::{ffi, str};

#[derive(Debug)]
pub enum SimStringError {
    Utf8Error(str::Utf8Error),
    CStrError(ffi::FromBytesUntilNulError),
}

impl From<str::Utf8Error> for SimStringError {
    fn from(value: str::Utf8Error) -> Self {
        Self::Utf8Error(value)
    }
}

impl From<ffi::FromBytesUntilNulError> for SimStringError {
    fn from(value: ffi::FromBytesUntilNulError) -> Self {
        Self::CStrError(value)
    }
}

/// A representation of SimConnect's strings.
///
/// It will usually be created by doing `ptr::read_unaligned(..)` in a struct
/// the string is contained in.
///
/// Example
///
///    #[repr(C, packed)]
///    struct MyStruct {
///        title: SimString<256>,
///        airport: SimString<32>,
///        // other sim data
///    }
#[derive(Clone, Debug)]
#[repr(C, packed)]
pub struct SimString<const N: usize>([u8; N]);

impl<const N: usize> SimString<N> {
    pub fn to_string(&self) -> Result<String, SimStringError> {
        let bytes = self.0;
        let c_str = ffi::CStr::from_bytes_until_nul(&bytes).unwrap();
        Ok(String::from(c_str.to_str()?))
    }
}

impl<const N: usize> std::fmt::Display for SimString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string().unwrap())
    }
}
