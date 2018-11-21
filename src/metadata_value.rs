use bytes::Bytes;
use http::header::HeaderValue;

use std::{cmp, fmt};
use std::error::Error;
use std::str::FromStr;
use std::marker::PhantomData;

use metadata_encoding::Ascii;
use metadata_encoding::Binary;
use metadata_encoding::ValueEncoding;
use metadata_key::MetadataKey;

/// Represents a custom metadata field value.
///
/// `MetadataValue` is used as the [`MetadataMap`] value.
///
/// [`HeaderMap`]: struct.HeaderMap.html
#[derive(Clone, Hash)]
#[repr(transparent)]
pub struct MetadataValue<VE: ValueEncoding> {
    // Note: There are unsafe transmutes that assume that the memory layout
    // of MetadataValue is identical to HeaderValue
    pub(crate) inner: HeaderValue,
    phantom: PhantomData<VE>,
}

/// A possible error when converting a `MetadataValue` from a string or byte
/// slice.
#[derive(Debug)]
pub struct InvalidMetadataValue {
    _priv: (),
}

/// A possible error when converting a `MetadataValue` from a string or byte
/// slice.
#[derive(Debug)]
pub struct InvalidMetadataValueBytes(InvalidMetadataValue);

/// A possible error when converting a `MetadataValue` to a string representation.
///
/// Metadata field values may contain opaque bytes, in which case it is not
/// possible to represent the value as a string.
#[derive(Debug)]
pub struct ToStrError {
    _priv: (),
}

pub type AsciiMetadataValue = MetadataValue<Ascii>;
pub type BinaryMetadataValue = MetadataValue<Binary>;

impl<VE: ValueEncoding> MetadataValue<VE> {
    /// Convert a static string to a `MetadataValue`.
    ///
    /// This function will not perform any copying, however the string is
    /// checked to ensure that no invalid characters are present. Only visible
    /// ASCII characters (32-127) are permitted.
    ///
    /// # Panics
    ///
    /// This function panics if the argument contains invalid metadata value
    /// characters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_static("hello");
    /// assert_eq!(val, "hello");
    /// ```
    #[inline]
    pub fn from_static(src: &'static str) -> Self {
        // TODO(pgron): Perform conversion
        MetadataValue {
            inner: HeaderValue::from_static(src),
            phantom: PhantomData,
        }
    }

    /// Attempt to convert a string to a `MetadataValue`.
    ///
    /// If the argument contains invalid metadata value characters, an error is
    /// returned. Only visible ASCII characters (32-127) are permitted. Use
    /// `from_bytes` to create a `MetadataValue` that includes opaque octets
    /// (128-255).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_str("hello").unwrap();
    /// assert_eq!(val, "hello");
    /// ```
    ///
    /// An invalid value
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_str("\n");
    /// assert!(val.is_err());
    /// ```
    #[inline]
    pub fn from_str(src: &str) -> Result<Self, InvalidMetadataValue> {
        // TODO(pgron): Perform conversion
        HeaderValue::from_str(src)
            .map(|value| MetadataValue {
                inner: value,
                phantom: PhantomData,
            })
            .map_err(|_| { InvalidMetadataValue { _priv: () } })
    }

    /// Converts a MetadataKey into a MetadataValue
    ///
    /// Since every valid MetadataKey is a valid MetadataValue this is done
    /// infallibly.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_name::<Ascii>("accept".parse().unwrap());
    /// assert_eq!(val, AsciiMetadataValue::from_bytes(b"accept").unwrap());
    /// ```
    #[inline]
    pub fn from_name<KeyVE: ValueEncoding>(name: MetadataKey<KeyVE>) -> Self {
        name.into()
    }

    /// Attempt to convert a byte slice to a `MetadataValue`.
    ///
    /// If the argument contains invalid metadata value bytes, an error is
    /// returned. Only byte values between 32 and 255 (inclusive) are permitted,
    /// excluding byte 127 (DEL).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_bytes(b"hello\xfa").unwrap();
    /// assert_eq!(val, &b"hello\xfa"[..]);
    /// ```
    ///
    /// An invalid value
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_bytes(b"\n");
    /// assert!(val.is_err());
    /// ```
    #[inline]
    pub fn from_bytes(src: &[u8]) -> Result<Self, InvalidMetadataValue> {
        // TODO(pgron): Perform conversion
        HeaderValue::from_bytes(src)
            .map(|value| MetadataValue {
                inner: value,
                phantom: PhantomData,
            })
            .map_err(|_| { InvalidMetadataValue { _priv: () } })
    }

    /// Attempt to convert a `Bytes` buffer to a `MetadataValue`.
    ///
    /// If the argument contains invalid metadata value bytes, an error is
    /// returned. Only byte values between 32 and 255 (inclusive) are permitted,
    /// excluding byte 127 (DEL).
    ///
    /// This function is intended to be replaced in the future by a `TryFrom`
    /// implementation once the trait is stabilized in std.
    #[inline]
    pub fn from_shared(src: Bytes) -> Result<Self, InvalidMetadataValueBytes> {
        // TODO(pgron): Perform conversion
        HeaderValue::from_shared(src)
            .map(|value| MetadataValue {
                inner: value,
                phantom: PhantomData,
            })
            .map_err(|_| {
                InvalidMetadataValueBytes(InvalidMetadataValue { _priv: () })
            })
    }

    /// Convert a `Bytes` directly into a `MetadataValue` without validating.
    ///
    /// This function does NOT validate that illegal bytes are not contained
    /// within the buffer.
    #[inline]
    pub unsafe fn from_shared_unchecked(src: Bytes) -> Self {
        // TODO(pgron): Perform conversion
        MetadataValue {
            inner: HeaderValue::from_shared_unchecked(src),
            phantom: PhantomData,
        }
    }

    /// Yields a `&str` slice if the `MetadataValue` only contains visible ASCII
    /// chars.
    ///
    /// This function will perform a scan of the metadata value, checking all the
    /// characters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_static("hello");
    /// assert_eq!(val.to_str().unwrap(), "hello");
    /// ```
    pub fn to_str(&self) -> Result<&str, ToStrError> {
        return self.inner.to_str().map_err(|_| { ToStrError { _priv: () } })
    }

    /// Returns the length of `self`.
    ///
    /// This length is in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_static("hello");
    /// assert_eq!(val.len(), 5);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the `MetadataValue` has a length of zero bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_static("");
    /// assert!(val.is_empty());
    ///
    /// let val = AsciiMetadataValue::from_static("hello");
    /// assert!(!val.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Converts a `MetadataValue` to a byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let val = AsciiMetadataValue::from_static("hello");
    /// assert_eq!(val.as_bytes(), b"hello");
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }

    /// Mark that the metadata value represents sensitive information.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut val = AsciiMetadataValue::from_static("my secret");
    ///
    /// val.set_sensitive(true);
    /// assert!(val.is_sensitive());
    ///
    /// val.set_sensitive(false);
    /// assert!(!val.is_sensitive());
    /// ```
    #[inline]
    pub fn set_sensitive(&mut self, val: bool) {
        self.inner.set_sensitive(val);
    }

    /// Returns `true` if the value represents sensitive data.
    ///
    /// Sensitive data could represent passwords or other data that should not
    /// be stored on disk or in memory. This setting can be used by components
    /// like caches to avoid storing the value. HPACK encoders must set the
    /// metadata field to never index when `is_sensitive` returns true.
    ///
    /// Note that sensitivity is not factored into equality or ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let mut val = AsciiMetadataValue::from_static("my secret");
    ///
    /// val.set_sensitive(true);
    /// assert!(val.is_sensitive());
    ///
    /// val.set_sensitive(false);
    /// assert!(!val.is_sensitive());
    /// ```
    #[inline]
    pub fn is_sensitive(&self) -> bool {
        self.inner.is_sensitive()
    }

    /// Converts a HeaderValue to a MetadataValue. This method assumes that the
    /// caller has made sure that the value is of the correct Ascii or Binary
    /// value encoding.
    #[inline]
    pub(crate) fn unchecked_from_header_value(value: HeaderValue) -> Self {
        MetadataValue { inner: value, phantom: PhantomData }
    }

    /// Converts a HeaderValue reference to a MetadataValue. This method assumes
    /// that the caller has made sure that the value is of the correct Ascii or
    /// Binary value encoding.
    // TODO(pgron): Rename to unchecked_
    #[inline]
    pub(crate) fn from_header_value_ref(header_value: &HeaderValue) -> &Self {
        unsafe { &*(header_value as *const HeaderValue as *const Self) }
    }

    /// Converts a HeaderValue reference to a MetadataValue. This method assumes
    /// that the caller has made sure that the value is of the correct Ascii or
    /// Binary value encoding.
    // TODO(pgron): Rename to unchecked_
    #[inline]
    pub(crate) fn from_mut_header_value_ref(header_value: &mut HeaderValue) -> &mut Self {
        unsafe { &mut *(header_value as *mut HeaderValue as *mut Self) }
    }
}

impl<VE: ValueEncoding> AsRef<[u8]> for MetadataValue<VE> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl<VE: ValueEncoding> fmt::Debug for MetadataValue<VE> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<KeyVE: ValueEncoding, ValueVE: ValueEncoding>
From<MetadataKey<KeyVE>> for MetadataValue<ValueVE> {
    #[inline]
    fn from(h: MetadataKey<KeyVE>) -> MetadataValue<ValueVE> {
        // TODO(pgron): Perform conversion
        MetadataValue {
            inner: h.inner.into(),
            phantom: PhantomData,
        }
    }
}

macro_rules! from_integers {
    ($($name:ident: $t:ident => $max_len:expr),*) => {$(
        impl<VE: ValueEncoding> From<$t> for MetadataValue<VE> {
            fn from(num: $t) -> MetadataValue<VE> {
                // TODO(pgron): Perform conversion
                MetadataValue {
                    inner: HeaderValue::from(num),
                    phantom: PhantomData,
                }
            }
        }

        #[test]
        fn $name() {
            let n: $t = 55;
            let val = AsciiMetadataValue::from(n);
            assert_eq!(val, &n.to_string());

            let n = ::std::$t::MAX;
            let val = AsciiMetadataValue::from(n);
            assert_eq!(val, &n.to_string());
        }
    )*};
}

from_integers! {
    // integer type => maximum decimal length

    // u8 purposely left off... AsciiMetadataValue::from(b'3') could be confusing
    from_u16: u16 => 5,
    from_i16: i16 => 6,
    from_u32: u32 => 10,
    from_i32: i32 => 11,
    from_u64: u64 => 20,
    from_i64: i64 => 20
}

#[cfg(target_pointer_width = "16")]
from_integers! {
    from_usize: usize => 5,
    from_isize: isize => 6
}

#[cfg(target_pointer_width = "32")]
from_integers! {
    from_usize: usize => 10,
    from_isize: isize => 11
}

#[cfg(target_pointer_width = "64")]
from_integers! {
    from_usize: usize => 20,
    from_isize: isize => 20
}

#[cfg(test)]
mod from_metadata_name_tests {
    //use super::*;
    use metadata_map::MetadataMap;

    #[test]
    fn it_can_insert_metadata_key_as_metadata_value() {
        let mut _map = MetadataMap::new();
/* TODO(pgron): Add me back
        map.insert("accept", MetadataKey::from_bytes(b"hello-world").unwrap().into());

        assert_eq!(
            map.get("accept").unwrap(),
            AsciiMetadataValue::from_bytes(b"hello-world").unwrap()
        );
        */
    }
}

impl<VE: ValueEncoding> FromStr for MetadataValue<VE> {
    type Err = InvalidMetadataValue;

    #[inline]
    fn from_str(s: &str) -> Result<MetadataValue<VE>, Self::Err> {
        MetadataValue::<VE>::from_str(s)
    }
}

impl<VE: ValueEncoding> From<MetadataValue<VE>> for Bytes {
    #[inline]
    fn from(value: MetadataValue<VE>) -> Bytes {
        Bytes::from(value.inner)
    }
}

impl<'a, VE: ValueEncoding> From<&'a MetadataValue<VE>> for MetadataValue<VE> {
    #[inline]
    fn from(t: &'a MetadataValue<VE>) -> Self {
        t.clone()
    }
}

impl fmt::Display for InvalidMetadataValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl Error for InvalidMetadataValue {
    fn description(&self) -> &str {
        "failed to parse metadata value"
    }
}

impl fmt::Display for InvalidMetadataValueBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for InvalidMetadataValueBytes {
    fn description(&self) -> &str {
        self.0.description()
    }
}

impl fmt::Display for ToStrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl Error for ToStrError {
    fn description(&self) -> &str {
        "failed to convert metadata to a str"
    }
}

// ===== PartialEq / PartialOrd =====

impl<VE: ValueEncoding> PartialEq for MetadataValue<VE> {
    #[inline]
    fn eq(&self, other: &MetadataValue<VE>) -> bool {
        self.inner == other.inner
    }
}

impl<VE: ValueEncoding> Eq for MetadataValue<VE> {}

impl<VE: ValueEncoding> PartialOrd for MetadataValue<VE> {
    #[inline]
    fn partial_cmp(&self, other: &MetadataValue<VE>) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<VE: ValueEncoding> Ord for MetadataValue<VE> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<VE: ValueEncoding> PartialEq<str> for MetadataValue<VE> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.inner == other.as_bytes()
    }
}

impl<VE: ValueEncoding> PartialEq<[u8]> for MetadataValue<VE> {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.inner == other
    }
}

impl<VE: ValueEncoding> PartialOrd<str> for MetadataValue<VE> {
    #[inline]
    fn partial_cmp(&self, other: &str) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(other.as_bytes())
    }
}

impl<VE: ValueEncoding> PartialOrd<[u8]> for MetadataValue<VE> {
    #[inline]
    fn partial_cmp(&self, other: &[u8]) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl<VE: ValueEncoding> PartialEq<MetadataValue<VE>> for str {
    #[inline]
    fn eq(&self, other: &MetadataValue<VE>) -> bool {
        *other == *self
    }
}

impl<VE: ValueEncoding> PartialEq<MetadataValue<VE>> for [u8] {
    #[inline]
    fn eq(&self, other: &MetadataValue<VE>) -> bool {
        *other == *self
    }
}

impl<VE: ValueEncoding> PartialOrd<MetadataValue<VE>> for str {
    #[inline]
    fn partial_cmp(&self, other: &MetadataValue<VE>) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl<VE: ValueEncoding> PartialOrd<MetadataValue<VE>> for [u8] {
    #[inline]
    fn partial_cmp(&self, other: &MetadataValue<VE>) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_bytes())
    }
}

impl<VE: ValueEncoding> PartialEq<String> for MetadataValue<VE> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        *self == &other[..]
    }
}

impl<VE: ValueEncoding> PartialOrd<String> for MetadataValue<VE> {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(other.as_bytes())
    }
}

impl<VE: ValueEncoding> PartialEq<MetadataValue<VE>> for String {
    #[inline]
    fn eq(&self, other: &MetadataValue<VE>) -> bool {
        *other == *self
    }
}

impl<VE: ValueEncoding> PartialOrd<MetadataValue<VE>> for String {
    #[inline]
    fn partial_cmp(&self, other: &MetadataValue<VE>) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl<'a, VE: ValueEncoding> PartialEq<MetadataValue<VE>> for &'a MetadataValue<VE> {
    #[inline]
    fn eq(&self, other: &MetadataValue<VE>) -> bool {
        **self == *other
    }
}

impl<'a, VE: ValueEncoding> PartialOrd<MetadataValue<VE>> for &'a MetadataValue<VE> {
    #[inline]
    fn partial_cmp(&self, other: &MetadataValue<VE>) -> Option<cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

impl<'a, VE: ValueEncoding, T: ?Sized> PartialEq<&'a T> for MetadataValue<VE>
    where MetadataValue<VE>: PartialEq<T>
{
    #[inline]
    fn eq(&self, other: &&'a T) -> bool {
        *self == **other
    }
}

impl<'a, VE: ValueEncoding, T: ?Sized> PartialOrd<&'a T> for MetadataValue<VE>
    where MetadataValue<VE>: PartialOrd<T>
{
    #[inline]
    fn partial_cmp(&self, other: &&'a T) -> Option<cmp::Ordering> {
        self.partial_cmp(*other)
    }
}

impl<'a, VE: ValueEncoding> PartialEq<MetadataValue<VE>> for &'a str {
    #[inline]
    fn eq(&self, other: &MetadataValue<VE>) -> bool {
        *other == *self
    }
}

impl<'a, VE: ValueEncoding> PartialOrd<MetadataValue<VE>> for &'a str {
    #[inline]
    fn partial_cmp(&self, other: &MetadataValue<VE>) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

#[test]
fn test_debug() {
    let cases = &[
        ("hello", "\"hello\""),
        ("hello \"world\"", "\"hello \\\"world\\\"\""),
        ("\u{7FFF}hello", "\"\\xe7\\xbf\\xbfhello\""),
    ];

    for &(value, expected) in cases {
        let val = AsciiMetadataValue::from_bytes(value.as_bytes()).unwrap();
        let actual = format!("{:?}", val);
        assert_eq!(expected, actual);
    }

    let mut sensitive = AsciiMetadataValue::from_static("password");
    sensitive.set_sensitive(true);
    assert_eq!("Sensitive", format!("{:?}", sensitive));
}
