/**
 * Implementation of the v2 parser for quizzes in the new textual format.
 *
 * Author:  Ian Fisher (iafisher@protonmail.com)
 * Version: September 2019
 */
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use super::quiz;


#[derive(Debug)]
struct QuestionAttribute {
    field: String,
    value: String,
    line: usize,
    // Is it preceded by a dash?
    dashed: bool,
}

type QuestionEntry = Vec<QuestionAttribute>;


#[derive(Debug)]
pub enum QuestionV2 {
    ShortAnswer { text: Vec<String>, answer: quiz::Answer },
    Flashcard { top: String, bottom: quiz::Answer },
    List { text: Vec<String>, answers: Vec<quiz::Answer>, ordered: bool },
}


#[derive(Debug)]
pub struct QuestionWrapper {
    question: QuestionV2,
    tags: Vec<String>,
}


pub fn parse(path: &PathBuf) -> Vec<QuestionWrapper> {
    let contents = fs::read_to_string(path).unwrap();
    let entries = read_file(reader);
    let mut questions = Vec::new();
    for entry in entries.iter() {
        if entry.len() < 2 {
            continue;
        }

        let mut wrapper = if entry[0].field == "q" {
            // Either a ShortAnswer or a List question.
            let mut text_variants = Vec::new();
            let mut answers = Vec::new();
            let mut i = 0;
            while i < entry.len() && !entry[i].dashed {
                if entry[i].field == "q" {
                    text_variants.push(entry[i].value.clone());
                } else {
                    answers.push(quiz::Answer { 
                        variants: split_answer(&entry[i].value)
                    });
                }
                i += 1;
            }

            let q = if answers.len() == 1 {
                QuestionV2::ShortAnswer {
                    text: vec![entry[0].value.clone()],
                    answer: answers[0].clone() ,
                }
            } else {
                QuestionV2::List {
                    text: vec![entry[0].value.clone()],
                    answers: answers,
                    ordered: false,
                }
            };

            QuestionWrapper { question: q, tags: Vec::new() }
        } else {
            // A Flashcard question.
            let q = QuestionV2::Flashcard {
                top: entry[0].field.clone(),
                bottom: quiz::Answer { variants: split_answer(&entry[0].value) },
            };
            QuestionWrapper { question: q, tags: Vec::new() }
        };

        println!("{:?}", wrapper);
        questions.push(wrapper);
    }
    questions
}


fn read_file(reader: &mut BufReader<File>) -> Vec<QuestionEntry> {
    let mut entries = Vec::new();

    loop {
        if let Some(entry) = read_entry(reader) {
            entries.push(entry);
        } else {
            break;
        }
    }

    entries
}


fn read_entry(reader: &mut BufReader<File>) -> Option<QuestionEntry> {
    let mut entry = QuestionEntry::new();
    loop {
        if let Some(line) = read_line(reader) {
            if line.len() == 0 {
                break;
            }

            if let Some(colon_pos) = line.find(":") {
                let (field, value) = line.split_at(colon_pos);

                let trimmed_value = value[1..].trim().to_string();
                if field.starts_with("- ") {
                    let trimmed_field = field[2..].trim().to_string();
                    entry.push(QuestionAttribute {
                        field: trimmed_field,
                        value: trimmed_value,
                        line: 0,
                        dashed: true,
                    });
                } else {
                    let trimmed_field = field.trim().to_string();
                    entry.push(QuestionAttribute {
                        field: trimmed_field,
                        value: trimmed_value,
                        line: 0,
                        dashed: false,
                    });
                }
            } else {
                // TODO: Return an error.
            }
        } else {
            if entry.len() > 0 {
                break;
            } else {
                return None;
            }
        }
    }
    Some(entry)
}


fn read_line(reader: &mut BufReader<File>) -> Option<String> {
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap() == 0 {
        return None;
    }

    line = line.trim().to_string();
    if line.starts_with("#") {
        // Move to the next line
        read_line(reader)
    } else {
        Some(line)
    }
}


fn split_answer(answer: &str) -> Vec<String> {
    answer.split("/").map(|w| w.to_string()).collect()
}
