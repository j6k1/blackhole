extern crate blackhole_compress;

use std::fs::File;
use std::io::{Read,BufReader};
use std::ops::Deref;
use blackhole_compression::BlackHole;
use blackhole_compression::stream::{StreamReader, StreamWriter};

#[test]
fn compression_and_uncompression() {
    let mut reader = BufReader::new(File::open("testdata/legal_moves.rs").unwrap());

    let mut sr = StreamReader::new(&mut reader);

    let mut bh = BlackHole::new();

    let mut o = Vec::new();

    let mut sw = StreamWriter::new(&mut o);

    bh.compression(&mut sr,&mut sw).unwrap();

    let size = sw.written_size();

    let mut reader = BufReader::new(File::open("testdata/legal_moves.rs").unwrap());

    let mut original = Vec::new();

    reader.read_to_end(&mut original).unwrap();

    println!("{}",original.len());
    println!("{}",size);

    let mut o = o.deref();
    let mut sr = StreamReader::new(&mut o);

    let mut uncompress = Vec::new();

    let mut sw = StreamWriter::new(&mut uncompress);

    bh.uncompression(&mut sr,&mut sw).unwrap();

    //println!("{}",String::from_utf8_lossy(&uncompress));

    assert_eq!(original,uncompress);
}