use std::io::{Read, Write};
use crate::error::{ReadError, WriteError};

pub struct StreamReader<'a,R> where R: Read +'a {
    reader:&'a mut R,
    buf: [u8; 256],
    buf_size: usize,
    current_index:usize,
    current_bits:usize
}
impl<'a,R> StreamReader<'a,R> where R: Read +'a {
    pub fn new(writer:&'a mut R) -> StreamReader<'a,R> {
        StreamReader {
            reader:writer,
            buf: [0;256],
            buf_size: 0,
            current_index:0,
            current_bits:0
        }
    }

    fn read_next(&mut self) -> Result<usize,ReadError> {
        self.buf_size = self.reader.read(&mut self.buf)?;

        self.current_index = 0;

        Ok(self.buf_size)
    }
    
    pub fn get_bit_from_lsb(&mut self) -> Result<u8,ReadError> {
        if self.current_index >= self.buf.len() {
            if self.read_next()? == 0 {
                Err(ReadError::UnexpectedEofError)
            } else {
                self.get_bit_from_lsb()
            }
        } else {
            let bit = if self.buf[self.current_index] & 1u8 << (self.current_bits as u8) == 0 {
                0
            } else {
                1
            };

            self.current_bits += 1;

            if self.current_bits >= 8 {
                self.current_index += 1;
                self.current_bits = 0;
            }

            Ok(bit)
        }
    }

    pub fn get_bits_from_lsb(&mut self, size:usize) -> Result<u8,ReadError> {
        if size > 8 {
            Err(ReadError::InvalidArgumentError(String::from("The specified read size is too long.")))
        } else {
            let mut bits = 0;

            for i in 0..size {
                bits |= self.get_bit_from_lsb()? & 1u8 << i;
            }
            Ok(bits)
        }
    }

    pub fn read_once(&mut self) -> Result<Option<u8>,ReadError> {
        if self.buf_size == 0 || self.current_index >= self.buf_size {
            if self.read_next()? == 0 {
                return Ok(None);
            }
        }

        if self.current_bits == 0 {
            let r = self.buf[self.current_index];

            self.current_index += 1;

            Ok(Some(r))
        } else {
            self.get_bits_from_lsb(8).map(|b| Some(b))
        }
    }

    pub fn read_u8(&mut self) -> Result<u8,ReadError> {
        Ok(self.read_once()?.ok_or(ReadError::UnexpectedEofError)?)
    }

    pub fn read_u16(&mut self) -> Result<u16,ReadError> {
        Ok(self.read_once()?.ok_or(ReadError::UnexpectedEofError)? as u16 |
               (self.read_once()?.ok_or(ReadError::UnexpectedEofError)? as u16) << 8
        )
    }

    pub fn read_u32(&mut self) -> Result<u32,ReadError> {
        Ok(self.read_once()?.ok_or(ReadError::UnexpectedEofError)? as u32 |
           (self.read_once()?.ok_or(ReadError::UnexpectedEofError)? as u32) <<  8 |
           (self.read_once()?.ok_or(ReadError::UnexpectedEofError)? as u32) << 16 |
           (self.read_once()?.ok_or(ReadError::UnexpectedEofError)? as u32) << 24
        )
    }

    pub fn read_u64(&mut self) -> Result<u64,ReadError> {
        Ok((self.read_u32()? as u64) | (self.read_u32()? as u64) << 32)
    }

    pub fn read_until(&mut self,size:usize) -> Result<Vec<u8>,ReadError> {
        let mut r = Vec::with_capacity(size);

        for _ in 0..size {
            r.push(self.read_once()?.ok_or(ReadError::UnexpectedEofError)?);
        }

        Ok(r)
    }

    pub fn skip_bits(&mut self,count:usize) -> Result<(),ReadError> {
        self.current_index += count / 8;
        self.current_bits += count % 8;

        Ok(())
    }
}
pub struct StreamWriter<'a,W> where W: Write +'a {
    writer:&'a mut W,
    buf: [u8; 256],
    current_index:usize,
    current_bits:usize,
    written_size:usize
}
impl<'a,W> StreamWriter<'a,W> where W: Write +'a {
    pub fn new(writer:&'a mut W) -> StreamWriter<'a,W> {
        StreamWriter {
            writer:writer,
            buf: [0;256],
            current_index:0,
            current_bits:0,
            written_size:0
        }
    }

    pub fn flush(&mut self) -> Result<(),WriteError> {
        let mut slice = &mut self.buf[0..self.current_index];
        let mut start_index = 0;

        let current_index = if self.current_bits == 0 {
            self.current_index
        } else {
            self.current_index + 1
        };

        while start_index < current_index {
            let size = self.writer.write(&mut slice)?;

            if size == 0 {
                return Err(WriteError::InvalidState(String::from("An error occurred in writing data.")))
            }

            self.written_size += size;

            start_index += size;

            slice = &mut slice[size..];
        }

        self.current_index = 0;
        self.current_bits = 0;

        Ok(())
    }

    pub fn write(&mut self, b:u8) -> Result<(),WriteError> {
        if self.current_index >= 256 {
            self.flush()?;
        }

        if self.current_bits == 0 {
            self.buf[self.current_index] = b;
            self.current_index += 1;

            Ok(())
        } else {
            self.write_u8_to_bits(b)
        }
    }

    pub fn write_bit(&mut self,b:bool) -> Result<(),WriteError> {
        if self.current_bits == 0 {
            self.buf[self.current_index] = 0;
        }

        if b {
            self.buf[self.current_index] = self.buf[self.current_index] | (1u8 << self.current_bits);
        }

        self.current_bits += 1;

        if self.current_bits >= 8 {
            self.current_index += 1;
            self.current_bits = 0;
        }

        Ok(())
    }

    pub fn write_u16(&mut self,value:u16) -> Result<(),WriteError> {
        self.write((value & 0xFF) as u8)?;
        self.write(((value >> 8) &0xFF) as u8)?;

        Ok(())
    }

    pub fn write_u32(&mut self,value:u32) -> Result<(),WriteError> {
        self.write((value & 0xFF) as u8)?;
        self.write(((value >> 8) & 0xFF) as u8)?;
        self.write(((value >> 16) & 0xFF) as u8)?;
        self.write(((value >> 24) & 0xFF) as u8)?;

        Ok(())
    }

    pub fn write_u64(&mut self,value:u64) -> Result<(),WriteError> {
        self.write_u32((value & 0xFFFFFFFF) as u32)?;
        self.write_u32(((value >> 32) & 0xFFFFFFFF) as u32)?;

        Ok(())
    }

    pub fn write_u8_to_bits(&mut self,value:u8) -> Result<(),WriteError> {
        for i in 0..8 {
            if value & (1u8 << i) != 0 {
                self.write_bit(true)?;
            } else {
                self.write_bit(false)?;
            }
        }

        Ok(())
    }

    pub fn pad_zeros(&mut self) -> Result<(),WriteError> {
        if self.current_bits > 0 {
            self.current_bits = 0;
            self.current_index += 1;
        }

        Ok(())
    }

    pub fn write_bytes(&mut self,bytes:&[u8]) -> Result<(),WriteError> {
        for &b in bytes.into_iter() {
            self.write(b)?;
        }

        Ok(())
    }

    pub fn written_size(&self) -> usize {
        self.written_size
    }
}
