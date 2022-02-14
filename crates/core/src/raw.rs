// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use crate::*;
use std::{
    ffi::{c_void, CString},
    thread,
    time::{Duration, Instant},
};

#[cfg(feature = "event")]
use crate::event::{listen, Event, Handler};

/// Tag Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TagId(pub(crate) i32);

/// wrapper of tag model based on `libplctag`
#[derive(Debug)]
pub struct RawTag {
    tag_id: i32,
}

impl RawTag {
    /// create new RawTag
    /// # Note
    /// if you passed wrong path parameters, your program might crash.
    /// you might want to use `PathBuilder` to build a path.
    ///
    /// # Examples
    /// ```rust,ignore
    /// use plctag::{RawTag};
    ///
    /// let path="protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16";
    /// let tag = RawTag::new(path, timeout).unwrap();
    /// ```
    pub fn new(path: impl AsRef<str>, timeout: u32) -> Result<Self> {
        let path = CString::new(path.as_ref()).unwrap();
        let tag_id = unsafe { ffi::plc_tag_create(path.as_ptr(), timeout as i32) };
        if tag_id < 0 {
            return Err(Status::new(tag_id));
        }
        Ok(Self { tag_id })
    }

    /// create new RawTag
    pub unsafe fn new_with_callback(
        path: impl AsRef<str>,
        timeout: u32,
        cb: Option<
            unsafe extern "C" fn(tag_id: i32, event: i32, status: i32, user_data: *mut c_void),
        >,
        user_data: *mut c_void,
    ) -> Result<Self> {
        let path = CString::new(path.as_ref()).unwrap();
        let tag_id = ffi::plc_tag_create_ex(path.as_ptr(), cb, user_data, timeout as i32);
        if tag_id < 0 {
            return Err(Status::new(tag_id));
        }
        Ok(Self { tag_id })
    }

    /// tag id
    #[inline(always)]
    pub fn id(&self) -> TagId {
        TagId(self.tag_id)
    }

    /// perform write operation.
    /// - blocking read if timeout > 0
    /// - non-blocking read if timeout = 0
    #[inline(always)]
    pub fn read(&self, timeout: u32) -> Status {
        let rc = unsafe { ffi::plc_tag_read(self.tag_id, timeout as i32) };
        rc.into()
    }

    /// perform write operation
    /// - blocking write if timeout > 0
    /// - non-blocking write if timeout = 0
    #[inline(always)]
    pub fn write(&self, timeout: u32) -> Status {
        let rc = unsafe { ffi::plc_tag_write(self.tag_id, timeout as i32) };
        rc.into()
    }

    /// wait until not pending, blocking
    /// # Note
    /// only for simple use cases
    #[inline]
    pub fn wait(&self, timeout: Option<Duration>) -> Status {
        let start = Instant::now();
        loop {
            if let Some(v) = timeout {
                if start.elapsed() > v {
                    return Status::Err(ffi::PLCTAG_ERR_TIMEOUT);
                }
            }

            let status = self.status();
            if !status.is_pending() {
                return status;
            }
            //sleep(Duration::from_millis(1));
            thread::yield_now();
        }
    }

    /// element size
    #[inline(always)]
    pub fn elem_size(&self) -> Result<i32> {
        self.get_attr("elem_size", 0)
    }

    /// element count
    #[inline(always)]
    pub fn elem_count(&self) -> Result<i32> {
        self.get_attr("elem_count", 0)
    }

    /// get tag attribute
    #[inline(always)]
    pub fn get_attr(&self, attr: impl AsRef<str>, default_value: i32) -> Result<i32> {
        let attr = CString::new(attr.as_ref()).unwrap();
        let val =
            unsafe { ffi::plc_tag_get_int_attribute(self.tag_id, attr.as_ptr(), default_value) };
        if val == i32::MIN {
            // error
            return Err(self.status());
        }
        Ok(val)
    }

    /// set tag attribute
    #[inline(always)]
    pub fn set_attr(&self, attr: impl AsRef<str>, value: i32) -> Result<()> {
        let attr = CString::new(attr.as_ref()).unwrap();
        let rc = unsafe { ffi::plc_tag_set_int_attribute(self.tag_id, attr.as_ptr(), value) };
        Status::new(rc).into_result()
    }

    /// poll tag status
    #[inline(always)]
    pub fn status(&self) -> Status {
        let rc = unsafe { ffi::plc_tag_status(self.tag_id) };
        Status::new(rc)
    }

    /// tag size in bytes
    #[inline(always)]
    pub fn size(&self) -> Result<u32> {
        let value = unsafe { ffi::plc_tag_get_size(self.tag_id) };
        if value < 0 {
            return Err(Status::from(value));
        }
        Ok(value as u32)
    }

    /// set tag size in bytes, returns old size
    #[inline(always)]
    pub fn set_size(&self, size: u32) -> Result<u32> {
        let value = unsafe { ffi::plc_tag_set_size(self.tag_id, size as i32) };
        if value < 0 {
            return Err(Status::from(value));
        }
        Ok(value as u32)
    }

    /// get bit value
    #[inline(always)]
    pub fn get_bit(&self, bit_offset: u32) -> Result<bool> {
        let val = unsafe { ffi::plc_tag_get_bit(self.tag_id, bit_offset as i32) };
        if val == i32::MIN {
            // error
            return Err(self.status());
        }
        Ok(val == 1)
    }

    /// set bit value
    #[inline(always)]
    pub fn set_bit(&self, bit_offset: u32, value: bool) -> Result<()> {
        let rc = unsafe {
            ffi::plc_tag_set_bit(self.tag_id, bit_offset as i32, if value { 1 } else { 0 })
        };
        Status::new(rc).into_result()
    }

    /// get bool value
    #[inline(always)]
    pub fn get_bool(&self, byte_offset: u32) -> Result<bool> {
        let value = self.get_u8(byte_offset)?;
        Ok(value > 0)
    }

    /// set bool value
    #[inline(always)]
    pub fn set_bool(&self, byte_offset: u32, value: bool) -> Result<()> {
        self.set_u8(byte_offset, if value { 1 } else { 0 })
    }

    /// get i8 value
    #[inline(always)]
    pub fn get_i8(&self, byte_offset: u32) -> Result<i8> {
        let val = unsafe { ffi::plc_tag_get_int8(self.tag_id, byte_offset as i32) };
        if val == i8::MIN {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// get i8 value
    #[inline(always)]
    pub fn set_i8(&self, byte_offset: u32, value: i8) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int8(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get u8 value
    #[inline(always)]
    pub fn get_u8(&self, byte_offset: u32) -> Result<u8> {
        let val = unsafe { ffi::plc_tag_get_uint8(self.tag_id, byte_offset as i32) };
        if val == u8::MAX {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set u8 value
    #[inline(always)]
    pub fn set_u8(&self, byte_offset: u32, value: u8) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint8(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get i16 value
    #[inline(always)]
    pub fn get_i16(&self, byte_offset: u32) -> Result<i16> {
        let val = unsafe { ffi::plc_tag_get_int16(self.tag_id, byte_offset as i32) };
        if val == i16::MIN {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set i16 value
    #[inline(always)]
    pub fn set_i16(&self, byte_offset: u32, value: i16) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int16(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get u16 value
    #[inline(always)]
    pub fn get_u16(&self, byte_offset: u32) -> Result<u16> {
        let val = unsafe { ffi::plc_tag_get_uint16(self.tag_id, byte_offset as i32) };
        if val == u16::MAX {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set u16 value
    #[inline(always)]
    pub fn set_u16(&self, byte_offset: u32, value: u16) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint16(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get i32 value
    #[inline(always)]
    pub fn get_i32(&self, byte_offset: u32) -> Result<i32> {
        let val = unsafe { ffi::plc_tag_get_int32(self.tag_id, byte_offset as i32) };
        if val == i32::MIN {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set i32 value
    #[inline(always)]
    pub fn set_i32(&self, byte_offset: u32, value: i32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int32(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get u32 value
    #[inline(always)]
    pub fn get_u32(&self, byte_offset: u32) -> Result<u32> {
        let val = unsafe { ffi::plc_tag_get_uint32(self.tag_id, byte_offset as i32) };
        if val == u32::MAX {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set u32 value
    #[inline(always)]
    pub fn set_u32(&self, byte_offset: u32, value: u32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint32(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get i64 value
    #[inline(always)]
    pub fn get_i64(&self, byte_offset: u32) -> Result<i64> {
        let val = unsafe { ffi::plc_tag_get_int64(self.tag_id, byte_offset as i32) };
        if val == i64::MIN {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set i64 value
    #[inline(always)]
    pub fn set_i64(&self, byte_offset: u32, value: i64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int64(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get u64 value
    #[inline(always)]
    pub fn get_u64(&self, byte_offset: u32) -> Result<u64> {
        let val = unsafe { ffi::plc_tag_get_uint64(self.tag_id, byte_offset as i32) };
        if val == u64::MAX {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set u64 value
    #[inline(always)]
    pub fn set_u64(&self, byte_offset: u32, value: u64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint64(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get f32 value
    #[inline(always)]
    pub fn get_f32(&self, byte_offset: u32) -> Result<f32> {
        let val = unsafe { ffi::plc_tag_get_float32(self.tag_id, byte_offset as i32) };
        if (val - f32::MIN).abs() <= f32::EPSILON {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set f32 value
    #[inline(always)]
    pub fn set_f32(&self, byte_offset: u32, value: f32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_float32(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// get f64 value
    #[inline(always)]
    pub fn get_f64(&self, byte_offset: u32) -> Result<f64> {
        let val = unsafe { ffi::plc_tag_get_float64(self.tag_id, byte_offset as i32) };
        if (val - f64::MIN).abs() <= f64::EPSILON {
            self.status().into_result()?;
        }
        Ok(val)
    }

    /// set f64 value
    #[inline(always)]
    pub fn set_f64(&self, byte_offset: u32, value: f64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_float64(self.tag_id, byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// Getting A String Length
    #[inline(always)]
    pub fn get_string_length(&self, byte_offset: u32) -> Result<u32> {
        let rc = unsafe { ffi::plc_tag_get_string_length(self.tag_id, byte_offset as i32) };
        if rc >= 0 {
            Ok(rc as u32)
        } else {
            Err(Status::new(rc))
        }
    }

    /// Getting A String Capacity
    #[inline(always)]
    pub fn get_string_capacity(&self, byte_offset: u32) -> Result<u32> {
        let rc = unsafe { ffi::plc_tag_get_string_capacity(self.tag_id, byte_offset as i32) };
        if rc >= 0 {
            Ok(rc as u32)
        } else {
            Err(Status::new(rc))
        }
    }

    /// Getting the Space Occupied by a String
    #[inline(always)]
    pub fn get_string_total_length(&self, byte_offset: u32) -> Result<u32> {
        let rc = unsafe { ffi::plc_tag_get_string_total_length(self.tag_id, byte_offset as i32) };
        if rc >= 0 {
            Ok(rc as u32)
        } else {
            Err(Status::new(rc))
        }
    }

    /// Reading A String
    #[inline(always)]
    pub fn get_string(&self, byte_offset: u32, buf: &mut [u8]) -> Result<()> {
        let rc = unsafe {
            ffi::plc_tag_get_string(
                self.tag_id,
                byte_offset as i32,
                buf.as_mut_ptr() as *mut i8,
                buf.len() as i32,
            )
        };
        Status::new(rc).into_result()
    }

    /// Write A String
    /// NOTE: panic if buf terminates with 0 byte
    #[inline(always)]
    pub fn set_string(&self, byte_offset: u32, buf: impl Into<Vec<u8>>) -> Result<()> {
        let buf = CString::new(buf).unwrap();
        let rc = unsafe { ffi::plc_tag_set_string(self.tag_id, byte_offset as i32, buf.as_ptr()) };
        Status::new(rc).into_result()
    }

    /// get raw bytes.
    /// If buffer length would exceed the end of the data in the tag data buffer, an out of bounds error is returned
    #[inline(always)]
    pub fn get_bytes_unchecked(&self, byte_offset: u32, buf: &mut [u8]) -> Result<usize> {
        let rc = unsafe {
            ffi::plc_tag_get_raw_bytes(
                self.tag_id,
                byte_offset as i32,
                buf.as_mut_ptr(),
                buf.len() as i32,
            )
        };
        Status::new(rc).into_result()?;
        Ok(buf.len())
    }

    /// get raw bytes
    #[inline]
    pub fn get_bytes(&self, byte_offset: u32, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let size = self.size()? as usize;
        if byte_offset as usize >= size {
            return Ok(0);
        }
        let slots_len = size - byte_offset as usize;
        let buf_len = std::cmp::min(slots_len, buf.len());
        let buf = &mut buf[..buf_len];
        self.get_bytes_unchecked(byte_offset, buf)
    }

    /// set raw bytes.
    /// If buffer length would exceed the end of the data in the tag data buffer, an out of bounds error is returned
    #[inline(always)]
    pub fn set_bytes_unchecked(&self, byte_offset: u32, buf: &[u8]) -> Result<usize> {
        let rc = unsafe {
            ffi::plc_tag_set_raw_bytes(
                self.tag_id,
                byte_offset as i32,
                buf.as_ptr() as *mut u8,
                buf.len() as i32,
            )
        };
        Status::new(rc).into_result()?;
        Ok(buf.len())
    }

    /// set raw bytes
    #[inline]
    pub fn set_bytes(&self, byte_offset: u32, buf: &[u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let size = self.size()? as usize;
        if byte_offset as usize >= size {
            return Ok(0);
        }
        let slots_len = size - byte_offset as usize;
        let buf_len = std::cmp::min(slots_len, buf.len());
        let buf = &buf[..buf_len];
        self.set_bytes_unchecked(byte_offset, buf)
    }

    /// note: registering a new callback will override existing one
    #[cfg(not(feature = "event"))]
    #[inline(always)]
    pub unsafe fn register_callback(
        &self,
        cb: Option<unsafe extern "C" fn(tag_id: i32, event: i32, status: i32)>,
    ) -> Status {
        //unregister first
        let _ = ffi::plc_tag_unregister_callback(self.tag_id);
        let rc = ffi::plc_tag_register_callback(self.tag_id, cb);
        rc.into()
    }

    /// note: registering a new callback will override existing one
    #[cfg(not(feature = "event"))]
    #[inline]
    pub unsafe fn register_callback_ex(
        &self,
        cb: Option<
            unsafe extern "C" fn(tag_id: i32, event: i32, status: i32, user_data: *mut c_void),
        >,
        user_data: *mut c_void,
    ) -> Status {
        //unregister first
        let _ = ffi::plc_tag_unregister_callback(self.tag_id);
        let rc = ffi::plc_tag_register_callback_ex(self.tag_id, cb, user_data);
        rc.into()
    }

    #[cfg(not(feature = "event"))]
    #[inline(always)]
    pub unsafe fn unregister_callback(&self) -> Status {
        let rc = ffi::plc_tag_unregister_callback(self.tag_id);
        rc.into()
    }

    /// listen for events
    ///
    /// # Examples
    /// ```rust,ignore
    /// use plctag::event::Event;
    /// let tag: RawTag = ...;
    /// let listener = tag.listen(|id, evt, status|
    /// {
    ///      println!("tag event: {}, status: {}", evt, status);   
    /// });
    ///
    /// //remove listener later
    /// drop(listener);
    /// ```
    #[cfg(feature = "event")]
    #[inline(always)]
    pub fn listen<F>(&self, f: F) -> Handler
    where
        F: FnMut(TagId, Event, Status) + Send + Sync + Clone + 'static,
    {
        listen(&self.tag_id, f)
    }

    /// Abort the pending operation.
    /// The operation is only needed when you write async code.
    /// For non-blocking read/write (timeout=0), it's your responsibility to call this method to cancel the pending
    /// operation when timeout or other necessary situations.
    #[inline(always)]
    pub fn abort(&self) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_abort(self.tag_id) };
        Status::new(rc).into_result()
    }

    /// get tag value of `T` that derives [`Decode`]
    #[cfg(feature = "value")]
    #[inline]
    pub fn get_value<T: Decode>(&self, byte_offset: u32) -> Result<T> {
        let v = T::decode(self, byte_offset)?;
        Ok(v)
    }

    /// set tag value that derives [`Encode`]
    #[cfg(feature = "value")]
    #[inline]
    pub fn set_value<T: Encode>(&self, byte_offset: u32, value: T) -> Result<()> {
        value.encode(self, byte_offset)
    }
}

impl Drop for RawTag {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            //let _ = self.abort();
            ffi::plc_tag_destroy(self.tag_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug() {
        let tag = RawTag::new("make=system&family=library&name=debug&debug=4", 100).unwrap();

        let size = tag.size().unwrap();
        assert!(size > 0);

        //read
        let res = tag.read(100);
        assert!(res.is_ok());
        let level = tag.get_u32(0).unwrap_or_default();
        assert_eq!(level, 4);

        //write
        let res = tag.set_u32(0, 1);
        assert!(res.is_ok());
        let res = tag.write(100);
        assert!(res.is_ok());

        //read
        let res = tag.read(100);
        assert!(res.is_ok());
        let level = tag.get_u32(0).unwrap_or_default();
        assert_eq!(level, 1);

        let mut buf: Vec<u8> = vec![0; size as usize];
        let size = tag.get_bytes(0, &mut buf).unwrap();
        assert_eq!(size, 30);
        let result = &[
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0,
        ];
        assert_eq!(&buf, result);

        buf[0] = 3;

        let count = tag.set_bytes(0, &buf[0..2]).unwrap();
        assert_eq!(count, 2);
        let count = tag.get_bytes(0, &mut buf[0..3]).unwrap();
        assert_eq!(count, 3);
        let result = &[3, 0, 0];
        assert_eq!(&buf[0..3], result);
    }
}
