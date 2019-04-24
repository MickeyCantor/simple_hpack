pub struct DynamicTable{
    table: Vec<(String,String)>,
    table_size: usize,
    current_size: usize,
}

impl DynamicTable {
    /// Builds a new dynamic table of a given size in bytes, fucntions as a FIFO list of headers as per [IETF RFC 7541 Section 4](https://tools.ietf.org/html/rfc7541#section-4)
    /// 
    /// ## Arguments
    /// 
    /// * dynamic_table_size - the size in bytes of the table
    /// 
    /// ## Returns
    /// 
    /// A new dynamic table with no values.
    pub fn new(dynamic_table_size: usize) -> DynamicTable {
        DynamicTable{table: Vec::new(), table_size: dynamic_table_size, current_size: 0}
    }

    /// Function that wraps the internal vector get call, Just to keep all the variables of the table private.
    pub fn get(&self, index: usize) -> Option<&(String, String)>{
        self.table.get(index)
    }

    /// Function used to add an entry to the dynamic table in FIFO format as per [IETF RFC 7541 Section 2.3](https://tools.ietf.org/html/rfc7541#section-2.3.2)
    /// 
    /// ## Arguments
    /// 
    /// * header - the Header you wish to insert into the dyamic table 
    /// 
    /// ## Returns
    /// 
    /// An error if the header is larger then the table size
    pub fn add(&mut self, header: (String,String)) -> Result<(),&'static str>{
        let header_size = header.0.capacity() + header.1.capacity() + 32;
        if header_size > self.table_size {
            Err("Header exceeds table size!")
        } else {
            println!("Adding header - {:?}, size - {}",header, header_size);
            let reamining_space = self.table_size - self.current_size;

            if reamining_space < header_size{
                println!("Removing header! header_size - {}, remaining_size - {}", header_size, reamining_space);
                self.reduce_size(self.table_size - header_size);
            }

            self.current_size = self.current_size + header_size;
            self.table.insert(0, header);
            Ok(())
        }
       
    }

    /// Function used to set the table size, removing any elements that need to be removed
    pub fn set_size(&mut self, new_size: usize){
        
        if new_size >= self.table_size {
            self.table_size = new_size;
        } else {
            self.table_size = new_size;
            self.reduce_size(new_size);
        }
    }

    /// Function used to reduce the size of the table to lessthan or equal to the given size, removing any elements from the end of the vector as needed 
    /// 
    /// ## Arguments
    /// 
    /// * new_size - the new size you wish to set the table to
    /// 
    /// ## Returns 
    /// 
    /// Nothing
    fn reduce_size(&mut self, new_size: usize){
        println!("cur size - {}, new size - {}", self.current_size, new_size);
        while self.current_size > new_size {
            let header = self.table.pop();
            println!("Removing - {:?}, cur size - {}", header, self.current_size);
            match header {
                Some(x) => self.current_size = self.current_size - (x.0.capacity() + x.1.capacity() + 32),
                None => panic!("Oh boy batman, i shouldent be here!")
            } 
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

     #[test]
    fn test_dynamic_table_add(){
        let mut table = DynamicTable::new(50);

        table.add((String::from("This"),String::from("Fits"))).unwrap();

        assert!(table.table.contains(&(String::from("This"), String::from("Fits"))))
    }

    #[test]
    fn test_dynamic_table_add_too_large(){
        let mut table = DynamicTable::new(10);

        assert!(table.add((String::from("This is too large!"), String::from("Still too long"))).is_err())
    }

    #[test]
    fn test_dynamic_table_add_removes_oldest(){
        let mut table = DynamicTable::new(83);

        table.add((String::from("Test"), String::from("Head"))).unwrap();
        table.add((String::from("Test"), String::from("Head2"))).unwrap();
        table.add((String::from("Test"), String::from("Head3"))).unwrap();

        assert!(!table.table.contains(&(String::from("Test"), String::from("Head"))));
        assert!(table.table.contains(&(String::from("Test"), String::from("Head2"))));
        assert!(table.table.contains(&(String::from("Test"), String::from("Head3"))));
    }

    #[test]
    fn test_dynamic_table_add_exact_size(){
        let mut table = DynamicTable::new(81);

        table.add((String::from("Test"), String::from("Head"))).unwrap();
        table.add((String::from("Test"), String::from("Head2"))).unwrap();

        assert!(table.table.contains(&(String::from("Test"), String::from("Head"))));
        assert!(table.table.contains(&(String::from("Test"), String::from("Head2"))));
    }

    #[test]
    fn test_dynamic_table_add_removes_oldest_to_exact_size(){
        let mut table = DynamicTable::new(82);

        table.add((String::from("Test"), String::from("Head"))).unwrap();
        table.add((String::from("Test"), String::from("Head2"))).unwrap();
        table.add((String::from("Test"), String::from("Head3"))).unwrap();

        assert!(!table.table.contains(&(String::from("Test"), String::from("Head"))));
        assert!(table.table.contains(&(String::from("Test"), String::from("Head2"))));
        assert!(table.table.contains(&(String::from("Test"), String::from("Head3"))));
    }

    #[test]
    fn test_dynamic_table_set_size_removes_oldest(){
        let mut table = DynamicTable::new(83);

        table.add((String::from("Test"), String::from("Head"))).unwrap();
        table.add((String::from("Test"), String::from("Head2"))).unwrap();        

        table.set_size(68);

        assert!(!table.table.contains(&(String::from("Test"), String::from("Head"))));
        assert!(table.table.contains(&(String::from("Test"), String::from("Head2"))));
    }

    #[test]
    fn test_dynamic_table_set_size_zero(){
        let mut table = DynamicTable::new(83);

        table.add((String::from("Test"), String::from("Head"))).unwrap();
        table.add((String::from("Test"), String::from("Head2"))).unwrap();

        table.set_size(0);

        assert!(table.table.is_empty());
    }
}