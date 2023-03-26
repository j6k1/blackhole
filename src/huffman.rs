use std::cmp::Ordering;
use std::collections::{BinaryHeap, BTreeMap};
use std::fmt::Debug;
use std::io::{Read, Write};
use std::ops::Deref;
use crate::error::{CompressionError, ReadError, UnCompressionError, WriteError};
use crate::num::Fraction;
use crate::stream::{StreamReader, StreamWriter};

#[derive(Debug,Clone)]
pub enum HuffmanNode<T> where T: Ord + Clone + Default {
    Node {
        left: Box<HuffmanNode<T>>,
        right: Box<HuffmanNode<T>>
    },
    Leaf {
        word: T
    }
}
impl<T> HuffmanNode<T> where T: Ord + Clone + Default {
    pub fn new(word:T) -> HuffmanNode<T> {
        HuffmanNode::Leaf {
            word: word
        }
    }

    pub fn empty() -> HuffmanNode<T> {
        HuffmanNode::Leaf { word: T::default() }
    }

    fn insert(self:Box<Self>,word:T,bits:Bits,index:usize) -> Result<Box<Self>,UnCompressionError> {
        match *self {
            HuffmanNode::Leaf { word: _} => {
                if index >= bits.len {
                    Ok(Box::new(HuffmanNode::Leaf { word: word }))
                } else {
                    let b = bits.get_bit(index)?;

                    Ok(if b == 0u8 {
                        Box::new(HuffmanNode::Node {
                            left: self.insert(word,bits,index+1)?,
                            right: Box::new(HuffmanNode::empty())
                        })
                    } else {
                        Box::new(HuffmanNode::Node {
                            left: Box::new(HuffmanNode::empty()),
                            right: self.insert(word,bits,index+1)?
                        })
                    })
                }
            },
            HuffmanNode::Node {
                left: l,
                right: r
            } => {
                if index < bits.len && bits.get_bit(index)? == 0 {
                    Ok(Box::new(HuffmanNode::Node {
                        left: l.insert(word,bits,index+1)?,
                        right: r
                    }))
                } else if index < bits.len && bits.get_bit(index)? == 1 {
                    Ok(Box::new(HuffmanNode::Node {
                        left: l,
                        right: r.insert(word,bits,index+1)?
                    }))
                } else {
                    Err(UnCompressionError::from(ReadError::InvalidState(String::from("Huffman node status is invalid."))))
                }
            }
        }
    }

    fn find_word<R>(&self,reader:&mut StreamReader<'_,R>) -> Result<&T,ReadError> where R: Read {
        match self {
            &HuffmanNode::Leaf { ref word} => {
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

    fn words(&self) -> Vec<&T> {
        match self {
            &HuffmanNode::Leaf { ref word } => {
                vec![word]
            },
            &HuffmanNode::Node { ref left, ref right } => {
                let mut words = Vec::new();
                let mut r = left.words();

                words.append(&mut r);

                let mut r = right.words();

                words.append(&mut r);

                words
            }
        }
    }
}
#[derive(Debug,Clone)]
pub struct HuffmanItem<T> where T: Ord + Clone + Default {
    node:HuffmanNode<T>,
    score:Fraction
}
impl<T> HuffmanItem<T> where T: Ord + Clone + Default {
    pub fn new(node:HuffmanNode<T>,score:Fraction) -> HuffmanItem<T> {
        HuffmanItem {
            node:node,
            score:score
        }
    }
}
impl<T> Ord for HuffmanItem<T> where T: Ord + Clone + Default {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score).reverse()
            .then((&self.node as *const HuffmanNode<T> as usize).cmp(&(&other.node as *const HuffmanNode<T> as usize)))
    }
}
impl<T> PartialOrd for HuffmanItem<T> where T: Ord + Clone + Default {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}
impl<T> PartialEq for HuffmanItem<T> where T: Ord + Clone + Default {
    fn eq(&self, other: &Self) -> bool {
        &self.node as *const HuffmanNode<T> as usize == &other.node as *const HuffmanNode<T> as usize
    }
}
impl<T> Eq for HuffmanItem<T> where T: Ord + Clone + Default {}
#[derive(Debug,Clone)]
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
            self.data[index] |= 1 << (self.len % 8);
        }

        self.len += 1;
    }

    pub fn get_bit(&self,index:usize) -> Result<u8,WriteError> {
        if self.data.len() <= index / 8 {
            Err(WriteError::InvalidState(String::from("Attempted to read outside the range of the input.")))
        } else {
            let bits = index % 8;

            Ok((self.data[index / 8] & (1 << bits)) >> bits)
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
#[derive(Debug)]
pub struct HuffmanTree<T> where T: Ord + Clone + Default + Debug {
    root:Option<Box<HuffmanNode<T>>>,
    dic:BTreeMap<T,Bits>
}
impl<T> HuffmanTree<T> where T: Ord + Clone + Default + Debug {
    pub fn new(words:Vec<(T,Fraction)>) -> HuffmanTree<T> {
        let mut queue = BinaryHeap::new();

        for (w,s) in words {
            queue.push(HuffmanItem::new(HuffmanNode::Leaf { word: w },s));
        }

        while queue.len() > 1 {
            let l = queue.pop().unwrap();
            let r = queue.pop().unwrap();

            let score = l.score + r.score;

            queue.push(HuffmanItem::new(HuffmanNode::Node {
                left: Box::new(l.node),
                right: Box::new(r.node)
            }, score))
        }

        let mut r = HuffmanTree {
            root: queue.pop().map(|item| Box::new(item.node)),
            dic: BTreeMap::new()
        };

        let mut dic = BTreeMap::new();

        r.root.as_ref().map(|root| {
            Self::build_dic(&mut dic, root, Bits::new());
        });

        r.dic = dic;

        r
    }

    pub fn empty() -> HuffmanTree<T> {
        HuffmanTree {
            root: None,
            dic: BTreeMap::new()
        }
    }

    fn build_dic(dic:&mut BTreeMap<T,Bits>, node: &Box<HuffmanNode<T>>, bits:Bits) {
        match &node.deref() {
            &HuffmanNode::Leaf { word } => {
                dic.insert(word.clone(),bits);
            },
            &HuffmanNode::Node {
                left,
                right
            } => {
                let mut lbits = bits.clone();
                let mut rbits = bits.clone();

                lbits.push_bit(false);
                rbits.push_bit(true);

                Self::build_dic(dic,&left,lbits);
                Self::build_dic(dic,&right,rbits);
            }
        }
    }

    pub fn insert(&mut self,word:T,bits:Bits) -> Result<(),UnCompressionError> {
        if let Some(root) = self.root.take() {
            self.root = Some(root.insert(word,bits,0)?);
        } else {
            let mut root = Box::new(HuffmanNode::empty());

            root = root.insert(word,bits,0)?;

            self.root = Some(root);
        }
        Ok(())
    }

    pub fn find_word<R>(&self,reader:&mut StreamReader<'_,R>) -> Result<&T,ReadError> where R: Read {
        if let Some(root) = &self.root {
            root.find_word(reader)
        } else {
            Err(ReadError::InvalidState(String::from("The Huffman tree is empty.")))
        }
    }

    pub fn write<'b,W>(&self,writer:&mut StreamWriter<'b,W>,word:T) -> Result<(),CompressionError> where W: Write {
        self.dic.get(&word)
            .ok_or(CompressionError::from(WriteError::InvalidState(String::from("No corresponding entry was found in the dictionary."))))
            .and_then(|bits | Ok(bits.write(writer)?))
    }

    pub fn words(&self) -> Vec<&T> {
        if let Some(root) = &self.root {
            root.words()
        } else {
            Vec::new()
        }
    }

    pub fn get_bits(&self,word:&T) -> Option<&Bits> {
        self.dic.get(word)
    }

    pub fn len(&self) -> usize {
        self.dic.len()
    }

    pub fn contains_word(&self,word:&T) -> bool {
        self.dic.contains_key(word)
    }
}