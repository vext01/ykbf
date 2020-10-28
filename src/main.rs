//#![feature(yk)]

use std::{
    env,
    ffi::CString,
    fs,
    io::{stderr, Write},
    path::Path,
    process,
    collections::HashMap
};

use ykcompile::{TraceCompiler, CompiledTrace};
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

struct IO {
    prg: Vec<u8>,
    map: Vec<usize>,
    pc: usize,
    ptr: usize,
    cells: Vec<u8>
}

impl IO {
    fn clone(&self) -> IO {
        IO {
            prg: self.prg.clone(),
            map: self.map.clone(),
            pc: self.pc.clone(),
            ptr: self.ptr.clone(),
            cells: self.cells.clone()
        }
    }
}

fn interp(prg: &[u8], map: Vec<usize>) {
    //let stdin = {
    //    let mode = CString::new("r").unwrap();
    //    unsafe { fdopen(libc::STDIN_FILENO, mode.as_ptr()) }
    //};

    let pc = 0;
    let ptr = 0;
    let mut cells = Vec::with_capacity(30000);
    cells.resize(30000, 0u8);

    let mut tio = IO { prg: prg.to_vec(), map: map, pc, ptr, cells: cells };

    //let mut lastop = None;
    let mut lasttraces: HashMap<u8, CompiledTrace<IO>> = HashMap::new();
    while tio.pc < prg.len() {
        // If there's a compiled trace, run it.
        let op = tio.prg[tio.pc];
        if let Some(ct) = lasttraces.get(&op) {
            // We got a trace. Now run it.
            let tmp_io = tio.clone();
            let success = ct.execute(&mut tio);
            if !success {
                // A guard failed. Reset IO state and return to interpreter mode.
                tio = tmp_io;
                lasttraces.remove(&op);
            } else {
                println!("{} {} {} {} (trace)", tio.prg[tio.pc] as char, tio.pc, tio.ptr, tio.cells[tio.ptr]);
            }
            continue;
        }
        let tr = start_tracing(TracingKind::HardwareTracing);
        interp_inner(&mut tio);

        let sir_trace = tr.stop_tracing().unwrap();
        let tir_trace = TirTrace::new(&*yktrace::sir::SIR, &*sir_trace).unwrap();
        let _comp_trace = TraceCompiler::<IO>::compile(tir_trace);
        lasttraces.insert(op, _comp_trace);
    }
}

#[interp_step]
fn interp_inner(
    //stdin: *mut libc::FILE, // FIXME needs to go into trace_inputs
    tio: &mut IO,
){
    match tio.prg[tio.pc] as char {
        '>' => {
            if tio.ptr == tio.cells.len() {
                tio.cells.push(0);
            }
            tio.ptr += 1;
        }
        '<' => {
            if tio.ptr > 0 {
                tio.ptr -= 1;
            }
        }
        '+' => {
            tio.cells[tio.ptr] = tio.cells[tio.ptr].wrapping_add(1);
        }
        '-' => {
            tio.cells[tio.ptr] = tio.cells[tio.ptr].wrapping_sub(1);
        }
        '.' => {
            let b = tio.cells[tio.ptr];
            unsafe {
                putchar(b as c_int);
            }
        }
        ',' => {
            //let v = unsafe { getchar() };
            //if v == libc::EOF {
            //    //if unsafe { ferror(stdin) } != 0 {
            //    //    panic!("Error when reading from stdin.");
            //    //}
            //} else {
            //    tio.cells[tio.ptr] = v as u8;
            //}
        }
        '[' => {
            if tio.cells[tio.ptr] == 0 {
                tio.pc = tio.map[tio.pc];
            }
        }
        ']' => {
            if tio.cells[tio.ptr] != 0 {
                tio.pc = tio.map[tio.pc];
            }
        }
        _ => unreachable!(),
    }
    tio.pc += 1;
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
