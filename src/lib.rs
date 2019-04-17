use std::collections::HashSet;
use lazy_static::lazy_static;
use std::str;

pub struct Hpack{
    dynamic_table: DynamicTable,
}

pub struct DynamicTable{
    table: Vec<(String,String)>,
    table_size: usize,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Header {
    value: (String, String),
    indexed: bool
}

impl DynamicTable {
    pub fn new(dynamic_table_size: usize) -> DynamicTable {
        DynamicTable{table: Vec::new(), table_size: dynamic_table_size}
    }

    /// Function used to add an entry to the dynamic table in FIFO format as per [IETF RFC 7541 Section 2.3](https://tools.ietf.org/html/rfc7541#section-2.3.2)
    /// 
     /// ## Arguments
    /// 
    /// * header - the Header you wish to insert into the dyamic table 
    /// 
    /// ## Returns
    /// 
    /// Nothing
    pub fn add(&mut self, header: (String,String)) {
        self.table.insert(0, header);
    }
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
    /// 
    /// 
    pub fn read_headers(&mut self, stream: Vec<u8>) -> Result<Vec<Header>,&'static str>{
        match stream.get(0) {  
            Some(x) => {
                if (x >> 7) == 1_u8 {
                    self.process_indexed(stream)
                }else if (x >> 6) == 1_u8{
                    self.process_indexed_literal(stream)
                }else if (x >> 4) == 0_u8 {
                    self.process_non_indexed_literal(stream)
                }else if (x >> 4) == 1_u8 {
                    self.process_never_indexed_literal(stream)
                } else {
                    Err("Write me 7!")
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

            println!("length - {}, remining vector - {:?}", length, stream);

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
            match self.dynamic_table.table.get(((i - 62) - 1) as usize){
                Some(x) => Ok((x.0.clone(), (x.1.clone()))),
                None => Err("Error index outside of dynamic table space"),
            }
        }
    }
}


/// Function that returns a new Indexed Header Field Representation as per [IETF RFC 7541 Section 6.1](https://tools.ietf.org/html/rfc7541#section-6.1)
/// 
/// ## Arguments 
/// 
/// * number - a 32 bit unsigned integer to be encoded larger then 0, represents the indexed position of the header 
/// 
/// ## Returns 
/// 
/// * Result<Vec<u8>,&'static str> - a result holding either the Vector of bytes or an error string
pub fn new_indexed(number: u32) -> Result<Vec<u8>,&'static str>{
    if number == 0 {
        Err(ERROR_INDEX_ZERO)
    }else{
        Ok(mask_first_byte(encode_int(7, number, Vec::new()), 128_u8))
    }
}

/// Function that returns a new Literal Header Field Representation with Incremental Indexing  as per [IETF RFC 7541 Section 6.2](https://tools.ietf.org/html/rfc7541#section-6.2)
/// 
/// ## Arguments 
/// 
/// * value - a string slice representing the value of the header to be encoded
/// * index - a number representing the indexed position of the header
/// * name - an optional string input, representing the name of the header referenced in the index table
/// * huffman - a boolean value representing if the string is huffman encoded or not
/// 
/// ## Returns
/// 
///  * Result<Vec<u8>,&'static str> - a result containing the Vector of bytes or an error string
pub fn new_literal(value: &str, index: u32, name: Option<&str>, _huffman: bool) -> Result<Vec<u8>, &'static str>{
    let build_literal = |index, value: &str| {
        if index == 0 {
            Err(ERROR_INDEX_ZERO)
        }else{
            let mut payload = encode_int(7, value.len() as u32,
                            mask_first_byte(encode_int(6, index, Vec::new()), 64_u8));
            payload.extend_from_slice(value.as_bytes());
            
            Ok(payload)
        }
    };

    let build_literal_with_name = |name: &str, value: &str| {
        let mut payload = encode_int(7, name.len() as u32, vec![64_u8]);
        payload.extend_from_slice(name.as_bytes());
        payload = encode_int(7, value.len() as u32, payload);
        payload.extend_from_slice(value.as_bytes());

        Ok(payload)
    };

    match name {
        Some(x) => build_literal_with_name(x, value),
        None => build_literal(index, value)
    }
}

/// Function that takes a Literal field and sets it to not be indexed 
/// 
/// ## Arguments
/// * self - the vector to be modified
/// 
/// ## Returns
/// * Vec<u8> - a Literal field that is not indexed
pub fn not_indexed(vec: Vec<u8>) -> Vec<u8>{
    let (int,mut vec) = decode_int(vec, 6);
    let mut re_encoded = encode_int(4, int, Vec::new());
    re_encoded.append(&mut vec);

    re_encoded
}

/// Function that takes a Literal field and sets it to never be indexed 
/// 
/// ## Arguments
/// * self - the vector to be modified
/// 
/// ## Returns
/// * Vec<u8> - a Literal field that is never indexed
pub fn never_indexed(vec: Vec<u8>) -> Vec<u8>{
    let (int,mut vec) = decode_int(vec, 6);
    let mut re_encoded =  mask_first_byte(encode_int(4, int, Vec::new()),16_u8);
    re_encoded.append(&mut vec);

    re_encoded
}

/// Function that encodes an integer using an ***n*** bytes leaving a prefix of ***8-n*** of zeros as per [IETF RFC 7541 Section 5.1](https://tools.ietf.org/html/rfc7541#section-5.1)
/// 
/// ## Arguments 
/// * n - the length of the prefix between 0..8
/// * number - the number to be encoded
/// * vec - a vector to store the number in, appends to the end of the vector
/// 
/// ## Returns
/// * Vec<u8> - a vector with the encoded number appended in bytes with the first byte always having a prefix of ***n*** zeros
fn encode_int (n: u32, number: u32,vec: Vec<u8>) -> Vec<u8> {
    let mut mut_vec = vec;
    if number as u32 <= (2_u32.pow(n)) - 1 {
        mut_vec.push(number as u8);
    }else{
        mut_vec = encode_int(n, (2_u32.pow(n)) - 1, mut_vec);
        let mut i = number - (2_u32.pow(n) - 1);
        while i >= 128 {
            mut_vec = encode_int(8, (i % 128) + 128, mut_vec);
            i = i / 128; 
        }
        mut_vec = encode_int(8, i, mut_vec);
    }

    mut_vec
}

/// Function that takes a stream of bytes represented as vector, and the number of bits encoded on **n** and decodes the integer, returning the number and the remaining byte stream
/// as per [IETF RFC 7541 Section 5.1](https://tools.ietf.org/html/rfc7541#section-5.1)
/// 
/// ## Arguments
/// * vec - the byte stream vector
/// * n - the encoded integer prefix
/// 
/// ## Returns
/// * (u32, Vec<u8>) - a tuple containing the decoded 32 bit integer, and a vector containing the remaining byte stream
fn decode_int(vec: Vec<u8>, n: u32) -> (u32, Vec<u8>) {
    let mut vec = vec;
    let mut int: u32 = (vec.remove(0) << (8-n) >> (8-n)) as u32;

    if int < 2_u32.pow(n) - 1 {
        (int, vec)
    }else{
        let mut m = 0;
        loop{
            let b = vec.remove(0);
            int = int + ((b & 127) as u32 * 2_u32.pow(m));
            m = m + 7;
            if (b & 128) != 128 {break}
        }
        (int, vec)
    }
} 

/// Function which masks the bits to one through a bitwise or function intended to be used
/// after the encode_int method to mask the ***n*** bit prefix with a binary encoding [(See IETF RFC 7541 Section 6)](https://tools.ietf.org/html/rfc7541#section-6)
/// 
/// ## Arguments
/// * vec - the vector of bytes to mask the first byte of, must be non empty
/// * mask - the mask to apply to the first byte
/// 
/// ## Returns
/// * Vec<u8> - a new vector with the first byte masked
fn mask_first_byte(vec: Vec<u8>, mask: u8) -> Vec<u8> {
    let mut vec = vec;
    let masked = vec.remove(0) | mask;
    
    vec.insert(0, masked);
    vec
}

static ERROR_INDEX_ZERO: &str = "Error - Indexed field cannot be zero";
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
mod tests {
    use super::*;

    #[test]
    fn test_encode_fits_in_prefix(){
        let int = encode_int(5, 10, Vec::new());

        assert_eq!(vec![10_u8], int);
    }

    #[test]
    fn test_encode_larger_then_prefix(){
        let int = encode_int(5,1337,Vec::new());

        assert_eq!(vec![31_u8, 154_u8, 10_u8],int);
    }

    #[test]
    fn test_new_indexed(){
        let int = new_indexed(1234).unwrap();

        assert_eq!(vec![255_u8,211_u8,8_u8], int);
    }

    #[test]
    fn test_new_indexed_zero(){
        let int = new_indexed(0).unwrap_err();

        assert_eq!(ERROR_INDEX_ZERO, int);
    }

    #[test]
    fn test_new_literal_string(){
        let literal = new_literal("This is 10",1, None, false).unwrap();

        assert_eq!(
            vec![65_u8,10_u8,0x54,0x68,0x69,0x73,0x20,0x69,0x73,0x20,0x31,0x30]
        , literal)
    }

    #[test]
    fn test_new_literal_string_zero_index(){
        let literal = new_literal("This is 10",0, None, false).unwrap_err();

        assert_eq!(ERROR_INDEX_ZERO, literal);
    }

    #[test]
    fn test_new_literal_with_name(){
        let literal = new_literal("This is 10",0, Some("Name"), false).unwrap();

        assert_eq!(
            vec![64_u8,4_u8,0x4E,0x61,0x6D,0x65,10_u8,0x54,0x68,0x69,0x73,0x20,0x69,0x73,0x20,0x31,0x30]
        , literal)
    }

    #[test]
    fn test_decode_fits_in_prefix(){
        let decoded = decode_int(vec![10_u8], 4);

        assert_eq!((10,Vec::new()),decoded);
    }

    #[test]
    fn test_decode_larger_then_prefix(){
        let decoded = decode_int(vec![31_u8, 154_u8, 10_u8], 5);

        assert_eq!((1337,Vec::new()), decoded);
    }

    #[test]
    fn test_decode_larger_then_prefix_with_remaining_bytes(){
         let decoded = decode_int(vec![65_u8,10_u8,0x54,0x68,0x69,0x73,0x20,0x69,0x73,0x20,0x31,0x30], 6);

        assert_eq!((1,vec![10_u8,0x54,0x68,0x69,0x73,0x20,0x69,0x73,0x20,0x31,0x30]), decoded);
    }

    #[test]
    fn test_new_literal_string_not_indexed(){
        let literal = not_indexed(new_literal("This is 10",1, None, false).unwrap());

        assert_eq!(
            vec![1_u8,10_u8,0x54,0x68,0x69,0x73,0x20,0x69,0x73,0x20,0x31,0x30]
        , literal)
    }

    #[test]
    fn test_new_literal_string_never_indexed(){
        let literal = never_indexed(new_literal("This is 10",1, None, false).unwrap());

        assert_eq!(
            vec![17_u8,10_u8,0x54,0x68,0x69,0x73,0x20,0x69,0x73,0x20,0x31,0x30]
        , literal)
    }

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
        hpack.read_headers(stream);

        let stream = vec![192_u8];

        assert_eq!("Error index outside of dynamic table space", hpack.read_headers(stream).unwrap_err());
    }

}

