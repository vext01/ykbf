#![feature(yk)]

use std::{
    env,
    ffi::CString,
    fs,
    io::{stderr, Write},
    path::Path,
    process,
};

use ykcompile::TraceCompiler;
use yktrace::{start_tracing, tir::TirTrace, TracingKind};

use getopts::Options;
use libc::{self, c_int, fdopen, ferror, getchar, putchar};

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

fn interp(prg: &[u8], map: Vec<usize>) {
    let stdin = {
        let mode = CString::new("r").unwrap();
        unsafe { fdopen(libc::STDIN_FILENO, mode.as_ptr()) }
    };

    let pc = 0;
    let ptr = 0;
    let mut cells = Vec::with_capacity(30000);
    cells.resize(30000, 0u8);
    let mut tio = core::yk::trace_inputs((pc, ptr, cells));

    while tio.0 < prg.len() {
        let tr = start_tracing(Some(TracingKind::HardwareTracing));
        interp_inner(prg, map, stdin, &mut tio);
        let sir_trace = tr.stop_tracing().unwrap();

        let tir_trace = TirTrace::new(&*sir_trace).unwrap();
        let _comp_trace = TraceCompiler::<&(usize, usize, Vec<u8>)>::compile(tir_trace);
        todo!("execute the trace");
    }
}

fn interp_inner(
    prg: &[u8],
    map: Vec<usize>,
    stdin: *mut libc::FILE,
    tio: &mut (usize, usize, Vec<u8>),
) {
    match prg[tio.0] as char {
        '>' => {
            if tio.1 == tio.2.len() {
                tio.2.push(0);
            }
            tio.1 += 1;
        }
        '<' => {
            if tio.1 > 0 {
                tio.1 -= 1;
            }
        }
        '+' => {
            tio.2[tio.1] = tio.2[tio.1].wrapping_add(1);
        }
        '-' => {
            tio.2[tio.1] = tio.2[tio.1].wrapping_sub(1);
        }
        '.' => {
            let b = tio.2[tio.1];
            unsafe {
                putchar(b as c_int);
            }
        }
        ',' => {
            let v = unsafe { getchar() };
            if v == libc::EOF {
                if unsafe { ferror(stdin) } != 0 {
                    panic!("Error when reading from stdin.");
                }
            } else {
                tio.2[tio.1] = v as u8;
            }
        }
        '[' => {
            if tio.2[tio.1] == 0 {
                tio.0 = map[tio.0];
            }
        }
        ']' => {
            if tio.2[tio.1] != 0 {
                tio.0 = map[tio.0];
            }
        }
        _ => unreachable!(),
    }
    tio.0 += 1;
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
