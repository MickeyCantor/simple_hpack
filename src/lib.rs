use std::str;

pub mod hpack;
pub mod dyn_table;

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
}

