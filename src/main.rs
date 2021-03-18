//#[cfg(not(tracermode = "hw"))]
//compile_error!("ykbf must be built with ykrustc");

use std::{
    env, fs,
    io::{stderr, Bytes, Read, Stdin, Stdout, Write},
    path::Path,
    process,
};

use getopts::Options;
use ykrt::{Location, MTBuilder};

fn usage(prog: &str) -> ! {
    let path = Path::new(prog);
    let leaf = path
        .file_name()
        .map(|x| x.to_str().unwrap_or("ykbf"))
        .unwrap_or("ykbf");
    writeln!(&mut stderr(), "Usage: {} [-h] <file.bf>", leaf).ok();
    process::exit(1)
}

fn compile(txt: &str) -> (String, Vec<usize>) {
    // BF comments are anything that's not a command character, so scrub all non-command
    // characters.
    let mut out = String::with_capacity(txt.len());
    for c in txt.chars() {
        match c {
            '>' | '<' | '+' | '-' | '.' | ',' | '[' | ']' => {
                out.push(c);
            }
            _ => (),
        }
    }
    out.shrink_to_fit();

    // Pre-calculate the [...] jumps as a map from PC -> PC.
    let mut map = vec![0; out.len()];
    let mut stack = Vec::new();
    for (i, c) in out.chars().enumerate() {
        if c == '[' {
            stack.push(i);
        } else if c == ']' {
            let s = stack.pop().unwrap();
            map[s] = i;
            map[i] = s;
        }
    }

    (out, map)
}

#[derive(Debug)]
struct InterpCtx {
    pc: usize,
    ptr: usize,
    cells: Vec<u8>,
    prog: Vec<u8>,
    map: Vec<usize>,
    stdin: Bytes<Stdin>,
    stdout: Stdout,
}

fn interp(prog: &[u8], map: Vec<usize>) {
    let mut icx = InterpCtx {
        pc: 0,
        ptr: 0,
        cells: Vec::from([0; 30000]),
        map,
        prog: prog.to_vec(),
        stdin: std::io::stdin().bytes(),
        stdout: std::io::stdout(),
    };

    // Locations are created for every program point, even if a loop cannot start there. We do this
    // because it's more efficient to store an unused `Location` than it is for elements to be of
    // type `Option<Location>`.
    let mut locs = Vec::with_capacity(icx.prog.len());
    locs.resize_with(prog.len(), Location::new);

    let mut mtt = MTBuilder::new().hot_threshold(2).init();
    loop {
        // We could pass a `Location` in for every program point, but that would cause traces to be
        // compiled that don't start at the beginning of a loop. Thus we only pass a `Location` for
        // instructions that are the first in a loop, passing in `None` otherwise.
        let loc = if icx.pc > 1 && (icx.prog[icx.pc - 1] as char) == '[' {
            Some(&locs[icx.pc])
        } else {
            None
        };
        if mtt.control_point(loc, interp_step, &mut icx) {
            break;
        }
    }
}

#[interp_step]
fn interp_step(icx: &mut InterpCtx) -> bool {
    match icx.prog[icx.pc] as char {
        '>' => {
            if icx.ptr == icx.cells.len() {
                icx.cells.push(0);
            }
            icx.ptr += 1;
        }
        '<' => {
            if icx.ptr > 0 {
                icx.ptr -= 1;
            }
        }
        '+' => {
            icx.cells[icx.ptr] = icx.cells[icx.ptr].wrapping_add(1);
        }
        '-' => {
            icx.cells[icx.ptr] = icx.cells[icx.ptr].wrapping_sub(1);
        }
        '.' => {
            icx.stdout.write_all(&[icx.cells[icx.ptr]]).unwrap();
        }
        ',' => {
            let v = icx.stdin.next();
            if let Some(b) = v {
                icx.cells[icx.ptr] = b.unwrap();
            }
        }
        '[' => {
            if icx.cells[icx.ptr] == 0 {
                icx.pc = icx.map[icx.pc];
            }
        }
        ']' => {
            if icx.cells[icx.ptr] != 0 {
                icx.pc = icx.map[icx.pc];
            }
        }
        _ => unreachable!(),
    }
    icx.pc += 1;
    if icx.pc >= icx.prog.len() {
        return true;
    }
    false
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog = &args[0];
    let matches = Options::new()
        .optflag("h", "help", "")
        .parse(&args[1..])
        .unwrap_or_else(|_| usage(prog));
    if matches.opt_present("h") || matches.free.len() != 1 {
        usage(prog);
    }

    let inv = fs::read(&matches.free[0]).expect("Can't read file.");
    let txt = String::from_utf8_lossy(&inv);
    let (prg, map) = compile(&txt);
    interp(prg.as_bytes(), map);
}
