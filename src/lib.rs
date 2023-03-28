extern crate rayon;

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::io::Write;
use std::io::Read;
use std::sync::Arc;

use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefIterator;

use crate::error::{ReadError, CompressionError, UnCompressionError};
use crate::huffman::{Bits, HuffmanTree};
use crate::stream::{StreamReader, StreamWriter};

pub mod error;
pub mod stream;
pub mod huffman;
pub mod num;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Score {
    word_len:usize,
    count:usize
}
impl Score {
    pub fn new(word_len:usize,count:usize) -> Score {
        Score {
            word_len,
            count
        }
    }
}
impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        self.word_len.cmp(&other.word_len).reverse().then(self.count.cmp(&other.count).reverse())
    }
}
impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}
#[derive(Debug)]
pub struct Word {
    word:Vec<u8>,
    score:Score,
    positions: BTreeSet<(usize,usize)>
}
impl Word {
    pub fn new(word:Vec<u8>, list: &[(usize,usize)], count:usize) -> Word {
        let mut positions = BTreeSet::new();

        for &(s,e) in list.iter() {
            positions.insert((s,e));
        }

        let word_len = word.len();

        Word {
            word: word,
            score: Score::new(word_len,count),
            positions: positions
        }
    }

    pub fn score(&self) -> Score {
        self.score
    }
}
impl Ord for Word {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score).then(self.word.cmp(&other.word).reverse())
    }
}
impl PartialOrd for Word {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}
impl PartialEq for Word {
    fn eq(&self, other: &Self) -> bool {
        self.word == other.word
    }
}
impl Eq for Word {

}
pub struct BlackHole {

}
impl BlackHole {
    pub fn new() -> BlackHole {
        BlackHole {

        }
    }

    pub fn analysis<'a,'b,R>(&self,reader:&'a mut StreamReader<'b,R>)
        -> Result<(BTreeSet<Word>,usize),CompressionError> where R: Read + 'b {

        let mut data = Vec::new();
        let mut words = BTreeSet::new();

        let mut list = Vec::new();

        let mut i = 0;

        {
            while let Some(b) = reader.read_once()? {
                data.push(b);
                list.push((i,i+1));

                i += 1;
            }
        }

        let mut dic = list.iter().fold(BTreeMap::new(), | mut acc,&(l,r) | {
            acc.entry(data[l..r].to_vec()).or_insert(Vec::new()).push((l,r));
            acc
        });

        for (word,list) in dic.iter() {
            let count = list.len();
            words.insert(Word::new(word.clone(), &list, count));
        }

        let data = Arc::new(data);
        let len = data.len();

        while dic.len() > 0 {
            let data = Arc::clone(&data);

            let (d,mut w) = dic.into_par_iter()
                .fold(|| (BTreeMap::new(),BTreeSet::new()), | (mut dic, mut words), (word, list) | {

                let next_iter = list.par_iter().copied().skip(1).chain(vec![(data.len(),data.len())].into_par_iter());

                let mut d = list.par_iter().filter(|&&(l,r)| {
                    r  < len && r - l - 1 > 0 && (word.len() * list.len() * list.len() / (r - l - 1) / (r - l - 1) <= r - l - 1)
                }).fold(|| BTreeMap::new(), | mut acc,&(l,r) | {
                    acc.entry(data[l..(r + 1)].to_vec()).or_insert(Vec::new()).push((l, r + 1));
                    acc
                }).reduce(|| BTreeMap::new(), | mut acc, mut t | {
                    acc.append(&mut t);
                    acc
                });

                let count =  list.par_iter().zip(next_iter).filter(|(a,b)| {
                    a.1 < b.0
                }).count();

                for (word,list) in d.iter() {
                    words.insert(Word::new(word.clone(), list, count));
                }

                dic.append(&mut d);

                (dic,words)
            }).reduce(|| (BTreeMap::new(),BTreeSet::new()), | (mut dic, mut words), (mut d, mut w) | {
                dic.append(&mut d);
                words.append(&mut w);

                (dic,words)
            });

            words.append(&mut w);
            dic = d;
        }

        Ok((words,data.len()))
    }

    pub fn build_words_and_tree<'a,'b>(&mut self,
                                       words:&'a BTreeSet<Word>,
                                       size:usize)
        -> Result<(Vec<Vec<u8>>,HuffmanTree<Vec<u8>>),CompressionError> where 'a: 'b {
        let mut seq = BTreeMap::new();

        let mut used_words = Vec::new();

        let mut start_to_end_map = BTreeMap::new();
        let mut end_to_start_map = BTreeMap::new();

        let mut current_size = 0;

        'outer: for w in words.into_iter() {
            let mut used_count = 0;

            for &(s,e) in w.positions.iter() {
                if current_size >= size {
                    break 'outer;
                }

                if start_to_end_map.range(..=s).next_back().map(|(_,&r)| s <= r).unwrap_or(false) {
                    continue;
                } else if end_to_start_map.range((e-1)..).next().map(|(_,&l)| e - 1 >= l).unwrap_or(false) {
                    continue;
                } else if start_to_end_map.range(s..).next().map(|(_,&l)| l <= e - 1).unwrap_or(false) {
                    continue;
                } else {
                    start_to_end_map.insert(s,e-1);
                    end_to_start_map.insert(e-1,s);

                    used_count += 1;

                    current_size += w.word.len();

                    seq.insert(s,w.word.clone());
                }
            }

            if used_count > 0 {
                used_words.push((w.word.clone(),used_count));
            }
        }

        let huffman_tree = HuffmanTree::new(used_words);

        let mut r = Vec::new();

        for (_,w) in seq.into_iter() {
            r.push(w);
        }
        Ok((r,huffman_tree))
    }

    pub fn complete_compression<W>(&mut self,writer:&mut StreamWriter<'_,W>,
                                   words:Vec<Vec<u8>>,
                                   huffman_tree:&mut HuffmanTree<Vec<u8>>)
        -> Result<(),CompressionError> where W: Write {
        for w in words {
            if !huffman_tree.contains_word(&w) {
                for &b in &w {
                    writer.write_bit(true)?;
                    writer.write(b)?;
                }
            } else {
                writer.write_bit(false)?;
                huffman_tree.write(writer,w)?;
            }
        }

        writer.pad_zeros()?;
        writer.flush()?;

        Ok(())
    }

    pub fn compression<W,R>(&mut self,reader:&mut StreamReader<'_,R>,writer:&mut StreamWriter<'_,W>)
        -> Result<(),CompressionError> where W: Write, R: Read {
        let (words,size) = self.analysis(reader)?;

        let (seq,mut huffman_tree) = self.build_words_and_tree(&words,size)?;

        let words = huffman_tree.words();

        let dic_size = words.len();

        if dic_size < 1 << 6 {
            writer.write((dic_size as u8) << 2)?;
        } else if dic_size < 1 << 14 {
            writer.write_u16(((dic_size as u16) << 2) | 0b01)?;
        } else if dic_size < 1 << 30 {
            writer.write_u32(((dic_size as u32) << 2) | 0b10)?;
        } else if dic_size < 1 << 62 {
            writer.write_u64(((dic_size as u64) << 2) | 0b11)?;
        } else {
            return Err(CompressionError::LimitError(String::from("Data size is too large.")))
        }

        for word in words {
            let bits = huffman_tree.get_bits(word).ok_or(ReadError::UnexpectedEofError)?;

            if bits.len() < 1 << 7 {
                writer.write_bit(false)?;
                writer.write_bits(bits.len() as u64,7)?;
            } else if bits.len() < 1 << 15 {
                writer.write_bit(true)?;
                writer.write_bits(bits.len() as u64,15)?;
            } else {
                return Err(CompressionError::LimitError(String::from("The size of the Huffman sign is too large.")));
            }

            bits.write(writer)?;

            let word_size = word.len();

            if word_size < 1 << 6 {
                writer.write((word_size as u8) << 2)?;
            } else if word_size < 1 << 14 {
                writer.write_u16(((word_size as u16) << 2) | 0b01)?;
            } else if word_size < 1 << 30 {
                writer.write_u32(((word_size as u32) << 2) | 0b10)?;
            } else if word_size < 1 << 62 {
                writer.write_u64(((word_size as u64) << 2) | 0b11)?;
            } else {
                return Err(CompressionError::LimitError(String::from("Data size is too large.")))
            }

            writer.write_bytes(word)?;
        }

        writer.write_u64(size as u64)?;

        self.complete_compression(writer,seq,&mut huffman_tree)
    }

    pub fn uncompression<R,W>(&mut self,reader:&mut StreamReader<'_,R>,writer:&mut StreamWriter<'_,W>)
        -> Result<(),UnCompressionError> where R: Read, W: Write {
        let h = reader.get_bits_from_lsb(2)?;

        let dic_size = if h == 0b00 {
            reader.get_bits_from_lsb(6)? as usize
        } else if h == 0b01 {
            (reader.get_bits_from_lsb(6)? as usize) | ((reader.read_u8()? as usize) << 6)
        } else if h == 0b10 {
            (reader.get_bits_from_lsb(6)? as usize) | ((reader.read_u8()? as usize) << 6) | ((reader.read_u16()? as usize) << 14)
        } else if h == 0b11 {
            (reader.get_bits_from_lsb(6)? as usize) |
            ((reader.read_u8()? as usize) << 6) |
            ((reader.read_u16()? as usize) << 14) |
            ((reader.read_u32()? as usize) << 30)
        } else {
            return Err(UnCompressionError::FormatError);
        };

        let mut huffman_tree = HuffmanTree::empty();

        for _ in 0..dic_size {
            let h = reader.get_bit_from_lsb()?;

            let huffman_code_size = if h == 0 {
                reader.get_bits_from_lsb(7)? as usize
            } else {
                reader.get_bits_from_lsb(7)? as usize | (reader.read_u8()? as usize) << 7
            };

            let mut code = Bits::new();

            for _ in 0..huffman_code_size {
                code.push_bit(if reader.get_bit_from_lsb()? == 0 {
                    false
                } else {
                    true
                });
            }

            let h = reader.get_bits_from_lsb(2)?;

            let word_size = if h == 0b00 {
                reader.get_bits_from_lsb(6)? as usize
            } else if h == 0b01 {
                (reader.get_bits_from_lsb(6)? as usize) | ((reader.read_u8()? as usize) << 6)
            } else if h == 0b10 {
                (reader.get_bits_from_lsb(6)? as usize) | ((reader.read_u8()? as usize) << 6) | ((reader.read_u16()? as usize) << 14)
            } else if h == 0b11 {
                (reader.get_bits_from_lsb(6)? as usize) |
                ((reader.read_u8()? as usize) << 6) |
                ((reader.read_u16()? as usize) << 14) |
                ((reader.read_u32()? as usize) << 30)
            } else {
                return Err(UnCompressionError::FormatError);
            };

            let word = reader.read_until(word_size)?;

            huffman_tree.insert(word,code)?;
        }

        let size = reader.read_u64()? as usize;

        let mut current_size = 0;

        while current_size < size {
            let h = reader.get_bit_from_lsb()?;

            if h == 0b0 {
                let word = huffman_tree.find_word(reader)?;
                current_size += word.len();

                writer.write_bytes(word)?;
            } else if h == 0b1 {
                current_size += 1;

                writer.write(reader.read_u8()?)?;
            } else {
                return Err(UnCompressionError::FormatError);
            };
        }

        writer.flush()?;

        Ok(())
    }
}