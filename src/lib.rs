use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::Write;
use std::io::Read;
use crate::error::{CompressionError};
use crate::huffman::HuffmanTree;
use crate::stream::{StreamReader, StreamWriter};

pub mod error;
pub mod stream;
pub mod huffman;

pub struct Word {
    word:Vec<u8>,
    count: usize,
    full_size: usize,
    positions: HashSet<usize>
}
impl Word {
    pub fn new(word:Vec<u8>, list: &[(usize,usize)], full_size:usize) -> Word {
        let mut positions = HashSet::new();

        for (s,_) in list.iter() {
            positions.insert(*s);
        }

        Word {
            word: word,
            count: list.len(),
            full_size: full_size,
            positions: positions
        }
    }

    pub fn add_count(&mut self) {
        self.count += 1;
    }

    pub fn push_position(&mut self, position: usize) {
        self.positions.insert(position);
    }

    pub fn contains_position(&self, position: usize) -> bool {
        self.positions.contains(&position)
    }

    pub fn size(&self) -> usize {
        self.word.len() + self.count
    }

    pub fn score(&self) -> u128 {
        (self.size() as u128) * self.full_size as u128 / (self.word.len() * self.count) as u128
    }
}
impl Ord for Word {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score().cmp(&other.score())
    }
}
impl PartialOrd for Word {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.score().cmp(&other.score()))
    }
}
impl PartialEq for Word {
    fn eq(&self, other: &Self) -> bool {
        self.score() == other.score()
    }
}
impl Eq for Word {

}
pub struct BlackHole {

}
impl BlackHole {
    pub fn analysis<'a,'b,R>(&self,reader:&'a mut StreamReader<'b,R>)
        -> Result<(BTreeSet<Word>,usize),CompressionError> where R: Read + 'b {

        let mut data = Vec::new();
        let mut words = BTreeSet::new();

        let mut list = Vec::new();

        let mut i = 0;

        {
            while let Some(b) = reader.read_u8()? {
                data.push(b);
                list.push((i,i+1));

                i += 1;
            }
        }

        let mut dic = list.iter().fold(BTreeMap::new(), |mut acc,&(l,r)| {
            acc.entry(data[l..r].to_vec()).or_insert(Vec::new()).push((l,r));
            acc
        });

        for (word,list) in dic.iter() {
            words.insert(Word::new(word.clone(), &list, data.len()));
        }

        while dic.len() > 0 {
            let (d,w) = dic.into_iter()
                .fold((BTreeMap::new(),BTreeSet::new()), | (mut dic, mut words),(_,list) | {
                    dic = list.iter()
                        .cloned()
                        .zip(list.iter().skip(1).chain(vec![(data.len(),data.len())].iter()))
                        .filter(|(a,b)| a.1 - 1 < b.0)
                        .map(|(a,_)| a)
                        .fold(dic,| mut acc, (l,next) | {
                            let next_list = list.iter().filter(|&&(_,r)| {
                                data[next] == data[r]
                            }).map(|&(l,r)| (l,r+1)).collect::<Vec<(usize,usize)>>();

                            if next_list.len() > 1 && next_list.len() + next - l + 1 <= list.len() + next - l {
                                acc.insert(data[l..(next+1)].to_vec(), next_list);
                            }

                            acc
                        });

                for (word,list) in dic.iter() {
                    words.insert(Word::new(word.clone(),list,data.len()));
                }

                (dic,words)
            });

            dic = d;
            words = w;
        }

        Ok((words,data.len()))
    }

    pub fn build_words_and_tree<'a,'b>(&mut self,words:&'a BTreeSet<Word>,size:usize,huffman_tree:&'b mut HuffmanTree)
        -> Result<Vec<Vec<u8>>,CompressionError> where 'a: 'b {
        let mut seq = Vec::new();

        let mut p = 0;

        'outer: while p < size {
            for w in words.iter() {
                if w.positions.contains(&p) {
                    huffman_tree.insert(w.word.clone())?;
                    seq.push(w.word.clone());
                    p +=  w.word.len();

                    continue 'outer;
                }
            }

            return Err(CompressionError::InvalidState(String::from("The word for the relevant position was not found in the dictionary.")));
        }

        Ok(seq)
    }

    pub fn complete_compression<W>(&mut self,writer:&mut StreamWriter<'_,W>,words:Vec<Vec<u8>>,huffman_tree:&mut HuffmanTree)
        -> Result<(),CompressionError> where W: Write {
        for w in words {
            huffman_tree.write(writer,w)?;
        }

        writer.flush()?;

        Ok(())
    }

    pub fn compression<W,R>(&mut self,reader:&mut StreamReader<'_,R>,writer:&mut StreamWriter<'_,W>)
        -> Result<(),CompressionError> where W: Write, R: Read {
        let (words,size) = self.analysis(reader)?;

        let mut huffman_tree = HuffmanTree::new();

        let seq = self.build_words_and_tree(&words,size,&mut huffman_tree)?;

        let dic = huffman_tree.get_dic();

        let dic_size = dic.len() - 1;

        if dic_size <= 1 << 6 {
            writer.write(0b00u8 << 6 | dic_size as u8)?;
        } else if dic_size <= 1 << 14 {
            writer.write(0b01u8 << 6 | (dic_size >> 8) as u8)?;
            writer.write(dic_size as u8 & 0xFFu8)?;
        } else if dic_size <= 1 << 30 {
            writer.write(0b10u8 << 6 | ((dic_size >> 24) as u8))?;
            writer.write((dic_size >> 16) as u8 & 0xFFu8)?;
            writer.write((dic_size >> 8) as u8 & 0xFFu8)?;
            writer.write(dic_size as u8 & 0xFFu8)?;
        } else {
            writer.write(0b11u8 << 6 | ((dic_size >> 56) as u8))?;
            writer.write((dic_size >> 48) as u8 & 0xFFu8)?;
            writer.write((dic_size >> 40) as u8 & 0xFFu8)?;
            writer.write((dic_size >> 32) as u8 & 0xFFu8)?;
            writer.write_u32((dic_size >> 32) as u32)?;
        }

        for (word,bits) in dic {
            if bits.len() <= 1 << 7 {
                writer.write(0b0u8 << 7 | (bits.len() as u8 - 1))?;
            } else if dic_size <= 1 << 15 {
                writer.write(0b01u8 << 7 | ((bits.len() >> 8) as u8 - 1))?;
                writer.write(dic_size as u8 & 0xFFu8)?;
            } else {
                return Err(CompressionError::InvalidState(String::from("The dictionary is too large.")))
            }

            bits.write(writer)?;
            writer.pad_zeros()?;
            writer.write_bytes(word.clone())?;
        }

        self.complete_compression(writer,seq,&mut huffman_tree)
    }
}