#![allow(unused_imports)]
#![allow(non_snake_case)]

use typstc::size::Size;
use typstc::syntax::*;
use Token::{
    Whitespace as W,
    LineComment as LC, BlockComment as BC, StarSlash as SS,
    LeftBracket as LB, RightBracket as RB,
    LeftParen as LP, RightParen as RP,
    LeftBrace as LBR, RightBrace as RBR,
    Colon as CL, Comma as CM, Equals as EQ, Expr as E,
    Star as ST, Underscore as U, Backtick as B, Text as T,
};

use Expression as Expr;
fn ID(ident: &str) -> Token { E(Expr::Ident(Ident::new(ident.to_string()).unwrap())) }
fn STR(ident: &str) -> Token { E(Expr::Str(ident.to_string())) }
fn SIZE(size: Size) -> Token<'static> { E(Expr::Size(size)) }
fn NUM(num: f64) -> Token<'static> { E(Expr::Num(num)) }
fn BOOL(b: bool) -> Token<'static> { E(Expr::Bool(b)) }


/// Parses the test syntax.
macro_rules! tokens {
    ($($src:expr =>($line:expr)=> $tokens:expr)*) => ({
        #[allow(unused_mut)]
        let mut cases = Vec::new();
        $(cases.push(($line, $src, $tokens.to_vec()));)*
        cases
    });
}

fn main() {
    let tests = include!("cache/parse");
    let mut errors = false;

    let len = tests.len();
    println!();
    println!("Running {} test{}", len, if len > 1 { "s" } else { "" });

    // Go through all test files.
    for (file, cases) in tests.into_iter() {
        print!("Testing: {}. ", file);

        let mut okay = 0;
        let mut failed = 0;

        // Go through all tests in a test file.
        for (line, src, expected) in cases.into_iter() {
            let found: Vec<_> = tokenize(src).map(Spanned::value).collect();

            // Check whether the tokenization works correctly.
            if found == expected {
                okay += 1;
            } else {
                if failed == 0 {
                    println!();
                }

                println!(" - Case failed in file {}.rs in line {}.", file, line);
                println!("   - Source:   {:?}", src);
                println!("   - Expected: {:?}", expected);
                println!("   - Found:    {:?}", found);

                failed += 1;
                errors = true;
            }
        }

        // Print a small summary.
        print!("{} okay, {} failed.", okay, failed);
        if failed == 0 {
            print!(" ✔")
        }
        println!();
    }

    println!();

    if errors {
        std::process::exit(-1);
    }
}