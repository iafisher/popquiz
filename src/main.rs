/**
 * Take a pop quiz from the command line.
 *
 * Author:  Ian Fisher (iafisher@protonmail.com)
 * Version: September 2019
 */
mod parser;
mod quiz;

use colored::*;

use quiz::QuizOptions;


fn main() {
    let options = quiz::parse_options();

    let result = match options {
        QuizOptions::Take(options) => {
            quiz::main_take(options)
        },
        QuizOptions::Count(options) => {
            quiz::main_count(options)
        },
        QuizOptions::Results(options) => {
            quiz::main_results(options)
        },
        QuizOptions::Edit(options) => {
            quiz::main_edit(options)
        },
        QuizOptions::Rm(options) => {
            quiz::main_rm(options)
        },
        QuizOptions::Mv(options) => {
            quiz::main_mv(options)
        },
        QuizOptions::Ls(options) => {
            quiz::main_ls(options)
        },
        QuizOptions::Path(options) => {
            quiz::main_path(options)
        },
        // Temporary
        QuizOptions::MigrateTest(options) => {
            println!("{:?}", parser::parse(&options.name));
            Ok(())
        },
    };

    if let Err(e) = result {
        if !quiz::is_broken_pipe(&e) {
            eprintln!("{}: {}", "Error".red(), e);
            ::std::process::exit(2);
        }
    }
}
