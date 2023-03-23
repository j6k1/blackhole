use std::collections::{BTreeMap};
use std::io::{Read, Write};
use crate::error::{ReadError, WriteError};
use crate::stream::{StreamReader, StreamWriter};

pub enum HuffmanNode {
    Node {
        left: Box<HuffmanNode>,
        right: Box<HuffmanNode>
    },
    Leaf {
        word: Vec<u8>
    }
}
impl HuffmanNode {
    pub fn new(word:Vec<u8>) -> HuffmanNode {
        HuffmanNode::Leaf {
            word: word
        }
    }

    fn insert(self:Box<Self>,
                    word:Vec<u8>,
                    mut lbits: Bits,
                    mut rbits: Bits,
                    dic:&mut BTreeMap<Vec<u8>,Bits>) -> Result<Box<Self>,WriteError> {

        match *self {
            HuffmanNode::Leaf { word: ref w} => {
                lbits.push_bit(false);
                rbits.push_bit(true);

                let w = w.clone();

                {
                    let w = w.clone();
                    let word = word.clone();

                    dic.insert(w, lbits.clone());
                    dic.insert(word, rbits.clone());
                }

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

    fn find_word<R>(&self,reader:&mut StreamReader<'_,R>) -> Result<&[u8],ReadError> where R: Read {
        match self {
            &HuffmanNode::Leaf { ref word } => {
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
pub struct HuffmanTree {
    root:Option<Box<HuffmanNode>>,
    dic:BTreeMap<Vec<u8>,Bits>
}
impl HuffmanTree {
    pub fn new() -> HuffmanTree {
        HuffmanTree {
            root: None,
            dic: BTreeMap::new()
        }
    }

    pub fn insert(&mut self, word: Vec<u8>) -> Result<(),WriteError> {
        if !self.dic.contains_key(&word) {
            let root = self.root.take();

            if let Some(r) = root {
                let lbits = Bits::new();
                let rbits = Bits::new();

                self.root = Some(r.insert(word,lbits,rbits,&mut self.dic)?);
            } else {
                self.root = Some(Box::new(HuffmanNode::new(word.clone())));

                let mut bits = Bits::new();

                bits.push_bit(false);

                self.dic.insert(word,bits);
            }
        }

        Ok(())
    }

    pub fn find_word<R>(&self,reader:&mut StreamReader<'_,R>) -> Result<&[u8],ReadError> where R: Read {
        if let Some(root) = &self.root {
            root.find_word(reader)
        } else {
            Err(ReadError::InvalidState(String::from("The Huffman tree is empty.")))
        }
    }

    pub fn write<'b,W>(&self,writer:&mut StreamWriter<'b,W>,word:Vec<u8>) -> Result<(),WriteError> where W: Write {
        self.dic.get(&word)
            .ok_or(WriteError::InvalidState(String::from("No corresponding entry was found in the dictionary.")))
            .and_then(|bits | bits.write(writer))
    }

    pub fn get_dic(&self) -> &BTreeMap<Vec<u8>,Bits> {
        &self.dic
    }
}