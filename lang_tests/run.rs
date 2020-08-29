use std::{env, path::PathBuf, process::Command};

use lang_tester::LangTester;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};

lazy_static! {
    static ref EXPECTED: Regex = RegexBuilder::new("^\\[(.*?)^\\][ \t]*$")
        .multi_line(true)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
}

fn main() {
    LangTester::new()
        .test_dir("lang_tests")
        .test_file_filter(|p| p.extension().unwrap().to_str().unwrap() == "bf")
        .test_extract(|s| {
            EXPECTED
                .captures(s)
                .map(|x| x.get(1).unwrap().as_str().trim().to_owned())
        })
        .test_cmds(|p| {
            // We call target/[debug|release]/yksom directly, because it's noticeably faster than
            // calling `cargo run`.
            let mut ykbf_bin = PathBuf::new();
            ykbf_bin.push(env!("CARGO_BIN_EXE_ykbf"));
            let mut vm = Command::new(ykbf_bin);
            vm.args(&[p.to_str().unwrap()]);
            vec![("VM", vm)]
        })
        .run();
}
