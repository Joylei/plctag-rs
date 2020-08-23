use crate::ffi;
use crate::{Result, Status};

use std::ffi::CString;

use std::thread::sleep;
use std::time::{Duration, Instant};

/// wrapper of tag of `libplctag`
///
pub struct RawTag {
    tag_id: i32,
}

impl RawTag {
    /// create new RawTag
    pub fn new(path: &str, timeout: u32) -> Result<Self> {
        let path = CString::new(path.to_owned()).unwrap();
        let tag_id = unsafe { ffi::plc_tag_create(path.as_ptr(), timeout as i32) };
        if tag_id < 0 {
            return Err(Status::new(ffi::PLCTAG_ERR_CREATE));
        }
        let mut tag = Self { tag_id };

        Ok(tag)
    }

    /// perform write operation.
    /// if timeout is 0, will not block; otherwise will block
    #[inline]
    pub fn read(&self, timeout: u32) -> Status {
        let rc = unsafe { ffi::plc_tag_read(self.tag_id, timeout as i32) };
        rc.into()
    }

    /// perform read operation
    #[inline]
    pub fn write(&self, timeout: u32) -> Status {
        let rc = unsafe { ffi::plc_tag_write(self.tag_id, timeout as i32) };
        rc.into()
    }

    /// wait until not pending
    #[inline]
    pub fn wait(&self) -> Status {
        loop {
            let status = self.status();
            if !status.is_pending() {
                return status;
            }
            sleep(Duration::from_millis(1));
        }
    }

    /// wait until not pending
    #[inline]
    pub fn wait_timeout(&self, timeout: u32) -> Status {
        let start = Instant::now();
        loop {
            if start.elapsed() > Duration::from_millis(timeout as u64) {
                return Status::Err(ffi::PLCTAG_ERR_TIMEOUT);
            }
            let status = self.status();
            if !status.is_pending() {
                return status;
            }
            sleep(Duration::from_millis(1));
        }
    }

    /// tag size of bytes
    #[inline]
    pub fn size(&self) -> Result<u32> {
        let value = unsafe { ffi::plc_tag_get_size(self.tag_id) };
        if value < 0 {
            return Err(Status::from(value));
        }
        Ok(value as u32)
    }

    /// element size
    pub fn element_size(&self) -> i32 {
        self.get_attr("elem_size", 0)
    }

    /// element count
    pub fn element_count(&self) -> i32 {
        self.get_attr("elem_count", 0)
    }

    /// get tag attribute
    pub fn get_attr(&self, attr: &str, default_value: i32) -> i32 {
        let attr = CString::new(attr).unwrap();
        unsafe { ffi::plc_tag_get_int_attribute(self.tag_id, attr.as_ptr(), default_value) }
    }

    /// set tag attribute
    pub fn set_attr(&self, attr: &str, value: i32) -> Status {
        let attr = CString::new(attr).unwrap();
        let rc = unsafe { ffi::plc_tag_set_int_attribute(self.tag_id, attr.as_ptr(), value) };
        rc.into()
    }

    pub unsafe fn register_callback(
        &self,
        cb: Option<unsafe extern "C" fn(tag_id: i32, event: i32, status: i32)>,
    ) -> Status {
        //unregister first
        let mut rc = ffi::plc_tag_unregister_callback(self.tag_id);
        info!("tag {} - unregister callback: {}", self.tag_id, rc);
        rc = ffi::plc_tag_register_callback(self.tag_id, cb);
        rc.into()
    }

    pub fn unregister_callback(&self) -> Status {
        let rc = unsafe { ffi::plc_tag_unregister_callback(self.tag_id) };
        rc.into()
    }

    /// abort the pending operation
    pub fn abort(&self) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_abort(self.tag_id) };
        Status::new(rc).as_result()
    }
}

impl TagId for RawTag {
    #[inline]
    fn id(&self) -> i32 {
        self.tag_id
    }
}

impl Accessor for RawTag {}

impl Drop for RawTag {
    fn drop(&mut self) {
        unsafe {
            //let _ = self.abort();
            ffi::plc_tag_destroy(self.tag_id);
        }
    }
}

#[doc(hidden)]
/// an object with id
pub trait TagId {
    fn id(&self) -> i32;
}

/// with `Accessor` trait, `RawTag` and `Proxy` share the same APIs
pub trait Accessor: TagId {
    #[inline]
    fn status(&self) -> Status {
        let rc = unsafe { ffi::plc_tag_status(self.id()) };
        Status::new(rc)
    }

    #[inline]
    fn get_bit(&self, bit_offset: u32) -> Result<bool> {
        let val = unsafe { ffi::plc_tag_get_bit(self.id(), bit_offset as i32) };
        if val == i32::MIN {
            // error
            return Err(self.status());
        }
        Ok(val == 1)
    }

    #[inline]
    fn set_bit(&self, bit_offset: u32, value: bool) -> Result<()> {
        let rc = unsafe {
            ffi::plc_tag_set_bit(self.id(), bit_offset as i32, if value { 1 } else { 0 })
        };
        Status::new(rc).as_result()
    }

    #[inline]
    fn get_bool(&self, byte_offset: u32) -> Result<bool> {
        let value = self.get_u8(byte_offset)?;
        Ok(value > 0)
    }

    #[inline]
    fn set_bool(&self, byte_offset: u32, value: bool) -> Result<()> {
        self.set_u8(byte_offset, if value { 1 } else { 0 })
    }

    #[inline]
    fn get_i8(&self, byte_offset: u32) -> Result<i8> {
        let val = unsafe { ffi::plc_tag_get_int8(self.id(), byte_offset as i32) };
        if val == i8::MIN {
            self.status().as_result()?;
        }
        Ok(val)
    }

    #[inline]
    fn set_i8(&self, byte_offset: u32, value: i8) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int8(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }

    #[inline]
    fn get_u8(&self, byte_offset: u32) -> Result<u8> {
        let val = unsafe { ffi::plc_tag_get_uint8(self.id(), byte_offset as i32) };
        if val == u8::MAX {
            self.status().as_result()?;
        }
        Ok(val)
    }

    #[inline]
    fn set_u8(&self, byte_offset: u32, value: u8) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint8(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
    #[inline]
    fn get_i16(&self, byte_offset: u32) -> Result<i16> {
        let val = unsafe { ffi::plc_tag_get_int16(self.id(), byte_offset as i32) };
        if val == i16::MIN {
            self.status().as_result()?;
        }
        Ok(val)
    }

    #[inline]
    fn set_i16(&self, byte_offset: u32, value: i16) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int16(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
    #[inline]
    fn get_u16(&self, byte_offset: u32) -> Result<u16> {
        let val = unsafe { ffi::plc_tag_get_uint16(self.id(), byte_offset as i32) };
        if val == u16::MAX {
            self.status().as_result()?;
        }
        Ok(val)
    }
    #[inline]
    fn set_u16(&self, byte_offset: u32, value: u16) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint16(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
    #[inline]
    fn get_i32(&self, byte_offset: u32) -> Result<i32> {
        let val = unsafe { ffi::plc_tag_get_int32(self.id(), byte_offset as i32) };
        if val == i32::MIN {
            self.status().as_result()?;
        }
        Ok(val)
    }
    #[inline]
    fn set_i32(&self, byte_offset: u32, value: i32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int32(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
    #[inline]
    fn get_u32(&self, byte_offset: u32) -> Result<u32> {
        let val = unsafe { ffi::plc_tag_get_uint32(self.id(), byte_offset as i32) };
        if val == u32::MAX {
            self.status().as_result()?;
        }
        Ok(val)
    }
    #[inline]
    fn set_u32(&self, byte_offset: u32, value: u32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint32(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
    #[inline]
    fn get_i64(&self, byte_offset: u32) -> Result<i64> {
        let val = unsafe { ffi::plc_tag_get_int64(self.id(), byte_offset as i32) };
        if val == i64::MIN {
            self.status().as_result()?;
        }
        Ok(val)
    }
    #[inline]
    fn set_i64(&self, byte_offset: u32, value: i64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_int64(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
    #[inline]
    fn get_u64(&self, byte_offset: u32) -> Result<u64> {
        let val = unsafe { ffi::plc_tag_get_uint64(self.id(), byte_offset as i32) };
        if val == u64::MAX {
            self.status().as_result()?;
        }
        Ok(val)
    }
    #[inline]
    fn set_u64(&self, byte_offset: u32, value: u64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_uint64(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
    #[inline]
    fn get_f32(&self, byte_offset: u32) -> Result<f32> {
        let val = unsafe { ffi::plc_tag_get_float32(self.id(), byte_offset as i32) };
        if (val - f32::MIN).abs() <= f32::EPSILON {
            self.status().as_result()?;
        }
        Ok(val)
    }
    #[inline]
    fn set_f32(&self, byte_offset: u32, value: f32) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_float32(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
    #[inline]
    fn get_f64(&self, byte_offset: u32) -> Result<f64> {
        let val = unsafe { ffi::plc_tag_get_float64(self.id(), byte_offset as i32) };
        if (val - f64::MIN).abs() <= f64::EPSILON {
            self.status().as_result()?;
        }
        Ok(val)
    }
    #[inline]
    fn set_f64(&self, byte_offset: u32, value: f64) -> Result<()> {
        let rc = unsafe { ffi::plc_tag_set_float64(self.id(), byte_offset as i32, value) };
        Status::new(rc).as_result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plc::shutdown;

    #[test]
    fn test_debug() {
        let res = RawTag::new("make=system&family=library&name=debug&debug=4", 100);
        assert!(res.is_ok());
        let tag = res.unwrap();

        let size = tag.size().unwrap();
        //assert_eq!(size, 4);

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
    }
}
