use crate::dyn_table::DynamicTable;
use crate::decode_int;
use lazy_static::lazy_static;
use std::str;

pub struct Hpack{
    dynamic_table: DynamicTable,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Header {
    value: (String, String),
    indexed: bool
}

impl Hpack{
    pub fn new(dynamic_table_size: usize) -> Hpack{
        Hpack{dynamic_table: DynamicTable::new(dynamic_table_size)}
    }

    ///Function used to read in a stream of headers, and convert them into a list of headers for consumption. 
    /// 
    /// ## Arguments
    /// 
    /// * stream - a vector of bytes used to represent the stream of headers being sent in
    /// 
    /// ## Returns
    /// 
    ///* Result<Vec<Header>,&'static str> - A vector of Header objects or an error message 
    /// 
    pub fn read_headers(&mut self, stream: Vec<u8>) -> Result<Vec<Header>,&'static str>{
        match stream.get(0) {  
            Some(x) => {
                if (x >> 7) == 1_u8 {
                    self.process_indexed(stream)
                }else if (x >> 6) == 1_u8{
                    self.process_indexed_literal(stream)
                }else if (x >> 5) == 1_u8{
                    let (size, stream) = decode_int(stream, 5);
                    self.dynamic_table.set_size(size as usize);
                    self.read_headers(stream)
                }else if (x >> 4) == 0_u8 {
                    self.process_non_indexed_literal(stream)
                }else if (x >> 4) == 1_u8 {
                    self.process_never_indexed_literal(stream)
                }else {
                    Err("Invalid start of header")
                }
            },
            None => Ok(Vec::new()),
        }
    }

    ///Function used to process an indexed refrence to a header from the static or dynamic table
    /// 
    /// ## Arguments
    /// 
    /// * stream - the vector of bytes to be consumed by the method 
    fn process_indexed(&mut self, stream: Vec<u8>) -> Result<Vec<Header>, &'static str> {
        let (int, stream) = decode_int(stream, 7);
        let mut vec = self.read_headers(stream)?;
        vec.insert(0, Header{value: self.get_static_entry_from_index(int)?, indexed: true});
        Ok(vec)
    }

    fn process_indexed_literal(&mut self, stream: Vec<u8>) -> Result<Vec<Header>, &'static str> {
        let (index, stream) = decode_int(stream, 6);
        
        if index == 0 {
            self.process_literial_with_name(stream, true)
        } else {
            self.process_literal_with_index(stream, index, true)
        }
    }

    fn process_non_indexed_literal(&mut self, stream: Vec<u8>) -> Result<Vec<Header>, &'static str> {
        let (index, stream) = decode_int(stream, 4);

         if index == 0 {
            self.process_literial_with_name(stream, true)
        } else {
            self.process_literal_with_index(stream, index, true)
        }
    }

    fn process_never_indexed_literal(&mut self, stream: Vec<u8>) -> Result<Vec<Header>, &'static str> {
        let (index, stream) = decode_int(stream, 4);

        if index == 0 {
            self.process_literial_with_name(stream, false)
        } else {
            self.process_literal_with_index(stream, index, false)
        }
    }

    fn get_string(stream: Vec<u8>) -> (Vec<u8>, String){
        let (length, mut stream) = decode_int(stream, 7);
            let range = length as usize;

            let value = match str::from_utf8(&stream.as_slice()[..range]) {
                Ok(x) => String::from(x),
                Err(_) => String::from("invalid utf8"),
            };

            for _ in 0..length {
                stream.remove(0);
            }

            (stream, value)
    }

    fn process_literial_with_name(&mut self, stream: Vec<u8>, indexed: bool) -> Result<Vec<Header>, &'static str> {
        let (stream, name) = Hpack::get_string(stream);
        let (stream, value) = Hpack::get_string(stream);

        let header = (name, String::from(value));
        if indexed {self.dynamic_table.add(header.clone());}

        let mut vec = self.read_headers(stream)?;
        vec.insert(0, Header{ value:header , indexed: indexed});

        Ok(vec)
    }

    fn process_literal_with_index(&mut self, stream: Vec<u8>, index: u32, indexed: bool) -> Result<Vec<Header>, &'static str> {
        let (stream, value) = Hpack::get_string(stream);

        let mut header = self.get_static_entry_from_index(index)?.clone();
        header.1 = value;
        if indexed {self.dynamic_table.add(header.clone());}

        let mut vec = self.read_headers(stream)?;

        vec.insert(0, Header{value: header, indexed: indexed});
        
        Ok(vec)
    }

    fn get_static_entry_from_index(&self, i: u32) -> Result<(String,String), &'static str> {
        if i < 62 {
            match STATIC_TABLE.get((i-1) as usize) {
                Some(x) => Ok((String::from(x.0),String::from(x.1))),
                None => Err("Error i is 0"),
            }
        } else {
            match self.dynamic_table.get(((i - 62) - 1) as usize){
                Some(x) => Ok((x.0.clone(), (x.1.clone()))),
                None => Err("Error index outside of dynamic table space"),
            }
        }
    }
}

lazy_static! {
    ///Static header list as defined by [IETF RFC 7541 Section 5.1](https://tools.ietf.org/html/rfc7541#appendix-A)
    static ref STATIC_TABLE: Vec<(&'static str,&'static str)> = {
        let mut table = Vec::new();
        table.push((":authority",""));
        table.push((":method","GET"));
        table.push((":method","POST"));
        table.push((":path","/"));
        table.push((":path","/index.html"));
        table.push((":scheme","http"));
        table.push((":scheme","https"));
        table.push((":status","200"));
        table.push((":status","204"));
        table.push((":status","206"));
        table.push((":status","304"));
        table.push((":status","400"));
        table.push((":status","404"));
        table.push((":status","500"));
        table.push(("accept-charset",""));
        table.push(("accept-encoding","gzip,deflate"));
        table.push(("accept-language",""));
        table.push(("accept-ranges",""));
        table.push(("accept",""));
        table.push(("access-control-allow-origin",""));
        table.push(("age",""));
        table.push(("allow",""));
        table.push(("authorization",""));
        table.push(("cache-control",""));
        table.push(("content-disposition",""));
        table.push(("content-encoding",""));
        table.push(("content-language",""));
        table.push(("content-length",""));
        table.push(("content-location",""));
        table.push(("contant-range",""));
        table.push(("content-type",""));
        table.push(("cookie",""));
        table.push(("date",""));
        table.push(("etag",""));
        table.push(("expect",""));
        table.push(("expires",""));
        table.push(("from",""));
        table.push(("host",""));
        table.push(("if-match",""));
        table.push(("if-modified-since",""));
        table.push(("if-none-match",""));
        table.push(("if-range",""));
        table.push(("if-unmodified-since",""));
        table.push(("last-modified",""));
        table.push(("link",""));
        table.push(("location",""));
        table.push(("max-forwards",""));
        table.push(("proxy-authenticate",""));
        table.push(("proxy-authorization",""));
        table.push(("range",""));
        table.push(("referer",""));
        table.push(("refresh",""));
        table.push(("retry-after",""));
        table.push(("server",""));
        table.push(("set-cookie",""));
        table.push(("strict-transport-security",""));
        table.push(("transfer-encoding",""));
        table.push(("user-agent",""));
        table.push(("vary",""));
        table.push(("via",""));
        table.push(("www-authenticate",""));
        table
    };
}

#[cfg(test)]
mod test{
    use super::*;

    #[test]
    fn test_read_headers_static_indexed(){
        let mut hpack = Hpack::new(128);

        let stream = vec![130_u8,132_u8];

        let expected = vec![Header{value: (String::from(":method"),String::from("GET")), indexed: true},
                            Header{value: (String::from(":path"),String::from("/")), indexed: true}];

        assert_eq!(expected,hpack.read_headers(stream).unwrap())
    }

    #[test]
    fn test_read_headers_literal_indexed(){
        let mut hpack = Hpack::new(128);

        let stream = vec![66_u8, 3_u8, 0x47, 0x45, 0x54, 79_u8, 3_u8, 0x73, 0x65, 0x74];

        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: true};
        let header_2 = Header{value: (String::from("accept-charset"),String::from("set")), indexed: true};

        let expected = vec![header_1.clone(), header_2.clone()];

        assert_eq!(expected, hpack.read_headers(stream).unwrap());
    }

    #[test]
    fn test_read_headers_literal_named(){
        let mut hpack = Hpack::new(128);

        let stream = vec![64_u8, 7_u8, 0x3a, 0x6d, 0x65, 0x74, 0x68, 0x6f, 0x64, 3_u8, 0x47, 0x45, 0x54, 64_u8, 14_u8, 0x61, 0x63, 0x63, 0x65, 0x70, 0x74, 0x2d, 0x63, 0x68, 0x61, 0x72, 0x73, 0x65, 0x74, 3_u8, 0x73, 0x65, 0x74];

        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: true};
        let header_2 = Header{value: (String::from("accept-charset"),String::from("set")), indexed: true};

        let expected = vec![header_1.clone(), header_2.clone()];

        assert_eq!(expected, hpack.read_headers(stream).unwrap());
    }

    #[test]
    fn test_read_headers_dynamic_literial_indexed(){
        let mut hpack = Hpack::new(128);

        let stream = vec![66_u8, 3_u8, 0x47, 0x45, 0x54, 79_u8, 3_u8, 0x73, 0x65, 0x74];

        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: true};

        hpack.read_headers(stream);

        let stream = vec![192_u8];
        let expected = vec![header_1.clone()];

        assert_eq!(expected,hpack.read_headers(stream).unwrap());
    }

    #[test]
    fn test_read_headers_dynamic_literial_named(){
        let mut hpack = Hpack::new(128);

        let stream = vec![64_u8, 7_u8, 0x3a, 0x6d, 0x65, 0x74, 0x68, 0x6f, 0x64, 3_u8, 0x47, 0x45, 0x54, 64_u8, 14_u8, 0x61, 0x63, 0x63, 0x65, 0x70, 0x74, 0x2d, 0x63, 0x68, 0x61, 0x72, 0x73, 0x65, 0x74, 3_u8, 0x73, 0x65, 0x74];

        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: true};
        let header_2 = Header{value: (String::from("accept-charset"),String::from("set")), indexed: true};

        hpack.read_headers(stream);

        let stream = vec![192_u8, 191_u8];
        let expected = vec![header_1.clone(), header_2.clone()];

        assert_eq!(expected,hpack.read_headers(stream).unwrap());
    }

    #[test]
    fn test_read_headers_literial_not_indexed_indexed(){
        let mut hpack = Hpack::new(128);
        let stream = vec![2_u8, 3_u8, 0x47, 0x45, 0x54];
        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: true};
        let expected = vec![header_1.clone()];

        assert_eq!(expected, hpack.read_headers(stream).unwrap());
    }

    #[test]
    fn test_read_headers_literial_not_indexed_named(){
        let mut hpack = Hpack::new(128);

        let stream = vec![0_u8, 7_u8, 0x3a, 0x6d, 0x65, 0x74, 0x68, 0x6f, 0x64, 3_u8, 0x47, 0x45, 0x54];

        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: true};

        let expected = vec![header_1.clone()];

        assert_eq!(expected, hpack.read_headers(stream).unwrap());
    }

    #[test]
    fn test_read_headers_literial_not_indexed_dosent_get_indexed(){
        let mut hpack = Hpack::new(128);
        let stream = vec![2_u8, 3_u8, 0x47, 0x45, 0x54];
        hpack.read_headers(stream);

        let stream = vec![192_u8];

        assert_eq!("Error index outside of dynamic table space", hpack.read_headers(stream).unwrap_err());
    }

    #[test]
    fn test_read_headers_literial_not_indexed_dosent_get_indexed_with_name(){
        let mut hpack = Hpack::new(128);
        let stream = vec![0_u8, 7_u8, 0x3a, 0x6d, 0x65, 0x74, 0x68, 0x6f, 0x64, 3_u8, 0x47, 0x45, 0x54];
        hpack.read_headers(stream);

        let stream = vec![192_u8];

        assert_eq!("Error index outside of dynamic table space", hpack.read_headers(stream).unwrap_err());
    }

    #[test]
    fn test_read_headers_literial_never_indexed_indexed(){
        let mut hpack = Hpack::new(128);
        let stream = vec![18_u8, 3_u8, 0x47, 0x45, 0x54];
        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: false};
        let expected = vec![header_1.clone()];

        assert_eq!(expected, hpack.read_headers(stream).unwrap());
    }

    #[test]
    fn test_read_headers_literial_never_indexed_named(){
        let mut hpack = Hpack::new(128);

        let stream = vec![16_u8, 7_u8, 0x3a, 0x6d, 0x65, 0x74, 0x68, 0x6f, 0x64, 3_u8, 0x47, 0x45, 0x54];

        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: false};

        let expected = vec![header_1.clone()];

        assert_eq!(expected, hpack.read_headers(stream).unwrap());
        
    }

    #[test]
    fn test_read_headers_literial_never_indexed_dosent_get_indexed(){
        let mut hpack = Hpack::new(128);
        let stream = vec![18_u8, 3_u8, 0x47, 0x45, 0x54];
        hpack.read_headers(stream);

        let stream = vec![192_u8];

        assert_eq!("Error index outside of dynamic table space", hpack.read_headers(stream).unwrap_err());
    }

    #[test]
    fn test_read_headers_literial_never_indexed_dosent_get_indexed_with_name(){
        let mut hpack = Hpack::new(128);
        let stream = vec![16_u8, 7_u8, 0x3a, 0x6d, 0x65, 0x74, 0x68, 0x6f, 0x64, 3_u8, 0x47, 0x45, 0x54];
        hpack.read_headers(stream).unwrap();

        let stream = vec![192_u8];

        assert_eq!("Error index outside of dynamic table space", hpack.read_headers(stream).unwrap_err());
    }

    #[test]
    fn test_change_table_size(){
        let mut hpack = Hpack::new(128);
        let stream = vec![63_u8, 154_u8, 10_u8, 2_u8, 3_u8, 0x47, 0x45, 0x54];
        let header_1 = Header{value: (String::from(":method"),String::from("GET")), indexed: true};
        let expected = vec![header_1.clone()];

        assert_eq!(expected,hpack.read_headers(stream).unwrap());
    }

}