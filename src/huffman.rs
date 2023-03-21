use std::collections::{HashMap};
use std::io::{Read, Write};
use crate::error::{ReadError, WriteError};
use crate::stream::{StreamReader, StreamWriter};

pub enum HuffmanNode<'a> {
    Node {
        left: Box<HuffmanNode<'a>>,
        right: Box<HuffmanNode<'a>>
    },
    Leaf {
        word: &'a [u8]
    }
}
impl<'a> HuffmanNode<'a> {
    pub fn new(word:&'a [u8]) -> HuffmanNode<'a> {
        HuffmanNode::Leaf {
            word: word
        }
    }

    fn insert(self:Box<Self>,
                    word:&'a [u8],
                    lbits: &mut Bits,
                    rbits: &mut Bits,
                    dic:&'a mut HashMap<&'a [u8],Bits>) -> Result<Box<Self>,WriteError> {

        match *self {
            HuffmanNode::Leaf { word: w} => {
                lbits.push_bit(false);
                rbits.push_bit(true);

                dic.insert(w,lbits.clone());
                dic.insert(word, rbits.clone());

                Ok(Box::new(HuffmanNode::Node {
                    left: Box::new(HuffmanNode::new(w)),
                    right: Box::new(HuffmanNode::new(word))
                }))
            },
            HuffmanNode::Node {
                left: l,
                right: r
            } => {
                lbits.push_bit(true);
                rbits.push_bit(true);

                Ok(Box::new(HuffmanNode::Node {
                    left: l,
                    right: r.insert(word,lbits,rbits,dic)?
                }))
            }
        }
    }

    fn find_word<'b,R>(&self,reader:&'b mut StreamReader<'b,R>) -> Result<&'a [u8],ReadError> where R: Read {
        match self {
            &HuffmanNode::Leaf { word } => {
                Ok(word)
            },
            &HuffmanNode::Node { ref left, ref right } => {
                if reader.get_bit_from_lsb()? == 0 {
                    left.find_word(reader)
                } else {
                    right.find_word(reader)
                }
            }
        }
    }
}
#[derive(Clone)]
pub struct Bits {
    len:usize,
    data:Vec<u8>
}
impl Bits {
    pub fn new() -> Bits {
        Bits {
            len:0,
            data:Vec::new()
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push_bit(&mut self,b:bool) {
        if self.data.len() <= self.len / 8 {
            self.data.push(0u8);
        }

        let index = self.data.len() - 1;

        if b {
            self.data[index] = self.data[index] | 1 << (7 - (self.len % 8));
        }

        self.len += 1;
    }

    pub fn get_bit(&self,index:usize) -> Result<u8,WriteError> {
        if self.data.len() <= index * 8 {
            Err(WriteError::InvalidState(String::from("Attempted to read outside the range of the input.")))
        } else {
            let bits = index % 8;

            Ok((self.data[index / 8] & 1 << (7 - bits)) >> (7 - bits))
        }
    }

    pub fn write<'a,W>(&self,writer:&mut StreamWriter<'a,W>) -> Result<(),WriteError> where W: Write {
        let len = self.len;

        for i in 0..len {
            writer.write_bit(if self.get_bit(i)? == 1 {
                true
            } else {
                false
            })?;
        }

        Ok(())
    }
}
pub struct HuffmanTree<'a> {
    root:Option<Box<HuffmanNode<'a>>>,
    dic:HashMap<&'a [u8],Bits>
}
impl<'a> HuffmanTree<'a> {
    pub fn new() -> HuffmanTree<'a> {
        HuffmanTree {
            root: None,
            dic: HashMap::new()
        }
    }

    pub fn insert(&'a mut self, word: &'a [u8]) -> Result<(),WriteError> {
        if !self.dic.contains_key(word) {
            let root = self.root.take();

            if let Some(r) = root {
                let mut lbits = Bits::new();
                let mut rbits = Bits::new();

                self.root = Some(r.insert(word,&mut lbits,&mut rbits,&mut self.dic)?);
            } else {
                self.root = Some(Box::new(HuffmanNode::new(word)));
            }
        }

        Ok(())
    }

    pub fn find_word<'b,R>(&'a self,reader:&'b mut StreamReader<'b,R>) -> Result<&'a [u8],ReadError> where R: Read {
        if let Some(root) = &self.root {
            root.find_word(reader)
        } else {
            Err(ReadError::InvalidState(String::from("The Huffman tree is empty.")))
        }
    }

    //pub fn write<'b,W>(&self,writer:&mut StreamWriter<'b,W>,word:&'a [u8]) -> Result<(),WriteError> where W: Write {
    //}
}