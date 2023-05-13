use std::{env, io};
use std::fs::File;
use std::io::{Read, Write};

const BUFFER_SIZE: usize = 409600;
const LMIN: u8 = 4;
const LMAX: u8 = 255;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: compress [input file] [output file]");
        std::process::exit(1);
    }

    let mut infile = File::open(&args[1])?;
    let mut buf = [0u8; BUFFER_SIZE];
    let fsize = infile.read(&mut buf)?;

    if fsize >= BUFFER_SIZE {
        eprintln!("Error: The input file is too large for compressing!");
        std::process::exit(3);
    } else if fsize <= 0 {
        eprintln!("Error: The input file is empty.");
        std::process::exit(3);
    }

    let mut outfile = File::create(&args[2])?;
    let mut bytes = [0u8; BUFFER_SIZE];
    let mut bi = 0usize;
    let mut pos = 0usize;

    while pos < fsize {
        let (mut l, mut ml) = (0u8, 0u8);
        let (mut p, mut mp) = (0usize, 0u16);

        while p < pos {
            if l >= LMAX {
                break;
            }

            if buf[p] == buf[pos + l as usize] {
                l += 1;
            } else {
                if l >= ml {
                    ml = l;
                    mp = p as u16;
                }

                p -= l as usize;
                l = 0;
            }

            p += 1;
        }

        if l >= ml {
            ml = l;
            mp = p as u16;
        }

        if ml >= LMIN {
            let mut bs = 0usize;
            while bi > 0 {
                let bx = if bi > 128 { 128 } else { bi };
                let b = 0b10000000 | (bx - 1) as u8;

                outfile.write_all(&[b])?;
                outfile.write_all(&bytes[bs..bs + bx])?;

                bi -= bx;
                bs += bx;
            }

            mp = mp.wrapping_sub(ml as u16) as usize as u16;

            mp = (mp >> 8) | (mp << 8);
            outfile.write_all(&mp.to_le_bytes())?;
            outfile.write_all(&[ml])?;

            pos += ml as usize;
        } else {
            bytes[bi] = buf[pos];
            bi += 1;
            pos += 1;
        }
        println!("Position: {}/{}", pos, fsize);
    }

    let mut bs = 0usize;
    while bi > 0 {
        let bx = if bi > 128 { 128 } else { bi };
        let b = 0b10000000 | (bx - 1) as u8;
        outfile.write_all(&[b])?;
        outfile.write_all(&bytes[bs..bs + bx])?;

        bi -= bx;
        bs += bx;
    }

    if pos == fsize {
        println!("Complete");
        return Ok(())
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other, 
            "Error: Failed to write"
        ))
    }
}