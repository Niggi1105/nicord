use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

use crate::error::FramingError;

///a simple frame
///[LEN, T as json with length = LEN]
///provides generic enframing and defraiming methods
///supports up to 10mb large frames
pub trait Frameable<T = Self>
where
    Self: Serialize + DeserializeOwned,
{
    fn deframe(bytes: &[u8]) -> Result<Option<Self>> {
        //leading 7 bytes are a string representation of the size as decimal
        if bytes.len() <= 7 {
            return Ok(None);
        }
        //convert
        let bstr = String::from_utf8(bytes.to_vec())?;
        let l = bstr[0..7].parse::<usize>()?;
        if bstr.len() < l + 7 {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice::<Self>(&bytes[7..l + 7])?))
    }
    fn enframe(&self) -> Result<Vec<u8>>
    where
        Self: Serialize,
    {
        let str = serde_json::to_string(self)?;
        if str.len() + 7 > 9_999_999 {
            return Err(FramingError::MaximumFrameSizeExceeded.into());
        }
        let mut r = String::new();
        let l = (str.len() as u32).to_string();
        if l.len() < 7 {
            let t = String::from_utf8(vec![b'0'; 7 - l.len()])?;
            r.push_str(&t);
        }
        r.push_str(&l);
        r.push_str(&str);
        Ok(r.into_bytes())
    }
}

#[cfg(test)]
mod test{
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct SomeTestStruct{
        str: String,
        int: i32,
        uint: u32,
        float: f64,
        vec: Vec<i64>,
    }
    impl Frameable for SomeTestStruct{}

    #[test]
    fn test_framing(){
        let s = SomeTestStruct {str: "Hello World".to_string(), int: -42, uint: 42, float: 42.42, vec: Vec::new()};
        let f = s.enframe().unwrap();
        println!("{:?}", f);
        let d = SomeTestStruct::deframe(&f).unwrap().unwrap();
        assert_eq!(s.str, d.str);
        assert_eq!(s.int, d.int);
        assert_eq!(s.uint, d.uint);
        assert_eq!(s.float, d.float);
        assert_eq!(s.vec, d.vec);
    }
}
