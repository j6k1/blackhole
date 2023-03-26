use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::io::Write;
use std::io::Read;
use crate::error::{CompressionError, UnCompressionError};
use crate::huffman::HuffmanTree;
use crate::stream::{StreamReader, StreamWriter};

pub mod error;
pub mod stream;
pub mod huffman;

#[derive(Debug)]
pub struct Word {
    word:Vec<u8>,
    count: usize,
    full_size: usize,
    positions: BTreeSet<(usize,usize)>
}
impl Word {
    pub fn new(word:Vec<u8>, list: &[(usize,usize)], full_size:usize) -> Word {
        let mut positions = BTreeSet::new();

        for &(s,e) in list.iter() {
            positions.insert((s,e));
        }

        Word {
            word: word,
            count: list.len(),
            full_size: full_size,
            positions: positions
        }
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
        (self.size() as u128 * self.full_size as u128 * other.word.len() as u128 * other.count as u128).cmp(
        &(other.size() as u128 * other.full_size as u128 * self.word.len() as u128 * self.count as u128)
        ).then(self.word.cmp(&other.word).reverse())
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
            words.insert(Word::new(word.clone(), &list, data.len()));
        }

        while dic.len() > 0 {
            let (d,w) = dic.into_iter()
                .fold((BTreeMap::new(),words), | (mut dic, mut words), (word, list) | {

                let next_iter = list.iter().copied().skip(1).chain(vec![(data.len(),data.len())].into_iter());

                let mut d = list.iter().zip(next_iter).filter(|&(a,b)| {
                    a.1 <= b.0
                }).map(|(a,_)| {
                    a
                }).filter(|&&(_,r)| {
                    r  < data.len()
                }).fold(BTreeMap::new(), | mut acc,&(l,r) | {
                    acc.entry(data[l..(r+1)].to_vec()).or_insert(Vec::new()).push((l,r+1));
                    acc
                }).into_iter().filter(|(_,next_list)| {
                    next_list.len() > 1 && next_list.len() + word.len() + 1 <= list.len() + word.len()
                }).fold(BTreeMap::new(),| mut acc,(k,v)| {
                    acc.insert(k,v);
                    acc
                });

                for (word,list) in d.iter() {
                    words.insert(Word::new(word.clone(), list, data.len()));
                }

                dic.append(&mut d);

                (dic,words)
            });

            dic = d;
            words = w;
        }

        Ok((words,data.len()))
    }

    pub fn build_words_and_tree<'a,'b>(&mut self,
                                       words:&'a BTreeSet<Word>,
                                       size:usize,
                                       huffman_tree:&'b mut HuffmanTree<Vec<u8>>)
        -> Result<Vec<Vec<u8>>,CompressionError> where 'a: 'b {
        let mut seq = BTreeMap::new();

        let mut used_words = BTreeSet::new();

        let mut start_to_end_map = BTreeMap::new();
        let mut end_to_start_map = BTreeMap::new();

        let mut current_size = 0;

        'outer: for w in words.into_iter() {
            let mut contains = false;

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

                    if w.word.len() > 1 {
                        contains = true;
                    }

                    current_size += w.word.len();

                    seq.insert(s,w.word.clone());
                }
            }

            if contains {
                used_words.insert(Word::new(w.word.clone(),&w.positions.iter().copied().collect::<Vec<(usize,usize)>>(),size));
            }
        }

        for w in used_words.into_iter() {
            if huffman_tree.len() + 1 < w.word.len() * 9 {
                huffman_tree.insert(w.word.clone())?;
            }
        }

        let mut r = Vec::new();

        for (_,w) in seq.into_iter() {
            r.push(w);
        }
        Ok(r)
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

        let mut huffman_tree = HuffmanTree::new();

        let seq = self.build_words_and_tree(&words,size,&mut huffman_tree)?;

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

        let mut huffman_tree = HuffmanTree::new();

        for _ in 0..dic_size {
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

            huffman_tree.insert(word)?;
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