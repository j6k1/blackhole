use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::Read;
use crate::error::ReadError;
use crate::stream::StreamReader;

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

    pub fn score(&self) -> u128 {
        (self.word.len() as u128 + self.count as u128) * self.full_size as u128 / (self.word.len() * self.count) as u128
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
        -> Result<BTreeSet<Word>,ReadError> where R: Read {

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
            let len = data.len();

            for &(l,r) in list.iter() {
                words.insert(Word::new(word.clone(), &list, data.len()));
            }
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

                            acc.insert(data[l..next].to_vec(),next_list);

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

        Ok(words)
    }
}