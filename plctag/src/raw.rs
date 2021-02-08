use crate::*;

//use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{ffi::CString, thread};

#[cfg(feature = "event")]
use crate::event::{Arc, Event, EventEmitter, ListenerBuilder};

/// wrapper of tag model based on `libplctag`
#[derive(Debug)]
pub struct RawTag {
    #[cfg(not(feature = "event"))]
    tag_id: i32,
    #[cfg(feature = "event")]
    inner: Arc<EventEmitter>,
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
            return Err(Status::new(ffi::PLCTAG_ERR_CREATE).into());
        }
        Ok(Self {
            #[cfg(not(feature = "event"))]
            tag_id,
            #[cfg(feature = "event")]
            inner: EventEmitter::new(tag_id),
        })
    }

    /// tag id in `libplctag`.
    ///
    /// # Note
    ///
    /// The id is not a resource handle.
    /// The id might be reused by `libplctag`. So if you use it somewhere, please take care.
    #[inline(always)]
    pub(crate) fn id(&self) -> i32 {
        #[cfg(not(feature = "event"))]
        {
            self.tag_id
        }
        #[cfg(feature = "event")]
        {
            self.inner.tag_id()
        }
    }

    /// perform write operation.
    /// - blocking read if timeout > 0
    /// - non-blocking read if timeout = 0
    #[inline(always)]
    pub fn read(&self, timeout: u32) -> Status {
        let rc = unsafe { ffi::plc_tag_read(self.id(), timeout as i32) };
        rc.into()
    }

    /// perform write operation
    /// - blocking write if timeout > 0
    /// - non-blocking write if timeout = 0
    #[inline(always)]
    pub fn write(&self, timeout: u32) -> Status {
        let rc = unsafe { ffi::plc_tag_write(self.id(), timeout as i32) };
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
            unsafe { ffi::plc_tag_get_int_attribute(self.id(), attr.as_ptr(), default_value) };
        if val == i32::MIN {
            // error
            return Err(self.status().into());
        }
        Ok(val)
    }

    /// set tag attribute
    #[inline(always)]
    pub fn set_attr(&self, attr: impl AsRef<str>, value: i32) -> Result<()> {
        let attr = CString::new(attr.as_ref()).unwrap();
        let rc = unsafe { ffi::plc_tag_set_int_attribute(self.id(), attr.as_ptr(), value) };
        Status::new(rc).into_result()
    }

    /// poll tag status
    #[inline(always)]
    pub fn status(&self) -> Status {
        let rc = unsafe { ffi::plc_tag_status(self.id()) };
        Status::new(rc)
    }

    ///Get the size in bytes used by the tag
    #[inline(always)]
    pub fn size(&self) -> Result<u32> {
        let value = unsafe { ffi::plc_tag_get_size(self.id()) };
        if value < 0 {
            return Err(Status::from(value).into());
        }
        Ok(value as u32)
    }

    #[inline(always)]
    pub fn get_bit(&self, bit_offset: u32) -> Result<bool> {
        let val = unsafe { ffi::plc_tag_get_bit(self.id(), bit_offset as i32) };
        if val == i32::MIN {
            // error
            return Err(self.status().into());
        }
        Ok(val == 1)
    }

    #[inline(always)]
    pub fn set_bit(&self, bit_offset: u32, value: bool) -> Result<()> {
        let rc = unsafe {
            ffi::plc_tag_set_bit(self.id(), bit_offset as i32, if value { 1 } else { 0 })
        };
        Status::new(rc).into_result()
    }

    #[inline(always)]
    pub fn get_bool(&self, byte_offset: u32) -> Result<bool> {
        let value = self.get_u8(byte_offset)?;
        Ok(value > 0)
    }

    #[inline(always)]
    pub fn set_bool(&self, byte_offset: u32, value: bool) -> Result<()> {
        self.set_u8(byte_offset, if value { 1 } else { 0 })
    }

    #[inline(always)]
    pub fn get_i8(&self, byte_offset: u32) -> Result<i8> {
        let val = unsafe { ffi::plc_tag_get_int8(self.id(), byte_offset as i32) };
        if val == i8::MIN {
            self.status().into_result()?;
        }
        Ok(val)
    }

    #[inline(always)]
    pub fn set_i8(&self, byte_offset: u32, value: i8) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int8(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    #[inline(always)]
    pub fn get_u8(&self, byte_offset: u32) -> Result<u8> {
        let val = unsafe { ffi::plc_tag_get_uint8(self.id(), byte_offset as i32) };
        if val == u8::MAX {
            self.status().into_result()?;
        }
        Ok(val)
    }

    #[inline(always)]
    pub fn set_u8(&self, byte_offset: u32, value: u8) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint8(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }
    #[inline(always)]
    pub fn get_i16(&self, byte_offset: u32) -> Result<i16> {
        let val = unsafe { ffi::plc_tag_get_int16(self.id(), byte_offset as i32) };
        if val == i16::MIN {
            self.status().into_result()?;
        }
        Ok(val)
    }

    #[inline(always)]
    pub fn set_i16(&self, byte_offset: u32, value: i16) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int16(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }
    #[inline(always)]
    pub fn get_u16(&self, byte_offset: u32) -> Result<u16> {
        let val = unsafe { ffi::plc_tag_get_uint16(self.id(), byte_offset as i32) };
        if val == u16::MAX {
            self.status().into_result()?;
        }
        Ok(val)
    }
    #[inline(always)]
    pub fn set_u16(&self, byte_offset: u32, value: u16) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint16(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }
    #[inline(always)]
    pub fn get_i32(&self, byte_offset: u32) -> Result<i32> {
        let val = unsafe { ffi::plc_tag_get_int32(self.id(), byte_offset as i32) };
        if val == i32::MIN {
            self.status().into_result()?;
        }
        Ok(val)
    }
    #[inline(always)]
    pub fn set_i32(&self, byte_offset: u32, value: i32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int32(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }
    #[inline(always)]
    pub fn get_u32(&self, byte_offset: u32) -> Result<u32> {
        let val = unsafe { ffi::plc_tag_get_uint32(self.id(), byte_offset as i32) };
        if val == u32::MAX {
            self.status().into_result()?;
        }
        Ok(val)
    }
    #[inline(always)]
    pub fn set_u32(&self, byte_offset: u32, value: u32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint32(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }
    #[inline(always)]
    pub fn get_i64(&self, byte_offset: u32) -> Result<i64> {
        let val = unsafe { ffi::plc_tag_get_int64(self.id(), byte_offset as i32) };
        if val == i64::MIN {
            self.status().into_result()?;
        }
        Ok(val)
    }
    #[inline(always)]
    pub fn set_i64(&self, byte_offset: u32, value: i64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int64(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }
    #[inline(always)]
    pub fn get_u64(&self, byte_offset: u32) -> Result<u64> {
        let val = unsafe { ffi::plc_tag_get_uint64(self.id(), byte_offset as i32) };
        if val == u64::MAX {
            self.status().into_result()?;
        }
        Ok(val)
    }
    #[inline(always)]
    pub fn set_u64(&self, byte_offset: u32, value: u64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint64(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }
    #[inline(always)]
    pub fn get_f32(&self, byte_offset: u32) -> Result<f32> {
        let val = unsafe { ffi::plc_tag_get_float32(self.id(), byte_offset as i32) };
        if (val - f32::MIN).abs() <= f32::EPSILON {
            self.status().into_result()?;
        }
        Ok(val)
    }
    #[inline(always)]
    pub fn set_f32(&self, byte_offset: u32, value: f32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_float32(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }
    #[inline(always)]
    pub fn get_f64(&self, byte_offset: u32) -> Result<f64> {
        let val = unsafe { ffi::plc_tag_get_float64(self.id(), byte_offset as i32) };
        if (val - f64::MIN).abs() <= f64::EPSILON {
            self.status().into_result()?;
        }
        Ok(val)
    }
    #[inline(always)]
    pub fn set_f64(&self, byte_offset: u32, value: f64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_float64(self.id(), byte_offset as i32, value) };
        Status::new(rc).into_result()
    }

    /// Note: it's not efficient
    pub fn get_bytes(&self, buf: &mut [u8]) -> Result<usize> {
        let size = self.size()? as usize;
        let mut i = 0;
        for item in buf {
            if i >= size {
                break;
            }
            *item = self.get_u8(i as u32)?;
            i += 1;
        }
        Ok(i)
    }

    /// Note: it's not efficient
    pub fn set_bytes(&self, buf: &[u8]) -> Result<usize> {
        let size = self.size()?;
        let len = std::cmp::min(buf.len(), size as usize);
        let buf = &buf[0..len];
        for (i, v) in buf.iter().enumerate() {
            self.set_u8(i as u32, *v)?;
        }
        Ok(len)
    }

    /// note: registering a new callback will override existing one
    #[cfg(not(feature = "event"))]
    #[inline(always)]
    pub unsafe fn register_callback(
        &self,
        cb: Option<unsafe extern "C" fn(tag_id: i32, event: i32, status: i32)>,
    ) -> Status {
        //unregister first
        let _ = ffi::plc_tag_unregister_callback(self.id());
        let rc = ffi::plc_tag_register_callback(self.id(), cb);
        rc.into()
    }

    #[cfg(not(feature = "event"))]
    #[inline(always)]
    pub fn unregister_callback(&self) -> Status {
        let rc = unsafe { ffi::plc_tag_unregister_callback(self.id()) };
        rc.into()
    }

    /// listen for events
    ///
    /// # Examples
    /// ```rust,ignore
    /// use plctag::event::Event;
    /// let tag: RawTag = ...;
    /// let listener = tag.listen(|evt, status|
    /// {
    ///      println!("tag event: {}, status: {}", evt, status);   
    /// })
    /// .event(Event::ReadCompleted)
    /// .manual(true)
    /// .on();
    ///
    /// //remove listener later
    /// listener.off();
    /// ```
    #[cfg(feature = "event")]
    #[inline(always)]
    pub fn listen<'a, F>(&'a self, f: F) -> ListenerBuilder<'a, F>
    where
        F: FnMut(Event, Status) + Send + 'static,
    {
        self.inner.listen(f)
    }

    /// Abort the pending operation.
    /// The operation is only needed when you write async code.
    /// For non-blocking read/write (timeout=0), it's your responsibility to call this method to cancel the pending
    /// operation when timeout or other necessary situations.
    #[inline(always)]
    pub fn abort(&self) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_abort(self.id()) };
        Status::new(rc).into_result()
    }
}

impl Drop for RawTag {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            //let _ = self.abort();
            ffi::plc_tag_destroy(self.id());
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
        let size = tag.get_bytes(&mut buf).unwrap();
        assert_eq!(size, 30);
        let result = &[
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0,
        ];
        assert_eq!(&buf, result);

        buf[0] = 3;

        tag.set_bytes(&buf).unwrap();

        tag.get_bytes(&mut buf).unwrap();
        let result = &[
            3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0,
        ];
        assert_eq!(&buf, result);
    }
}
