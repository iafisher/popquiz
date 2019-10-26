/**
 * Implementation of the core quiz data structures.
 *
 * Author:  Ian Fisher (iafisher@protonmail.com)
 * Version: October 2019
 */
use std::mem;
use std::time;

use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Serialize, Deserialize};

use super::common::{Location, Result, QuizError, TakeOptions};
use super::repetition;
use super::shell::CmdUI;


/// Represents an entire quiz.
#[derive(Debug)]
pub struct Quiz {
    pub instructions: Option<String>,
    pub questions: Vec<Box<dyn Question>>,
}


impl Quiz {
    pub fn take(
        &mut self, ui: &mut CmdUI, options: &TakeOptions) -> Result<QuizResult> {

        if options.flip {
            for q in self.questions.iter_mut() {
                q.flip();
            }
        }

        let mut results = Vec::new();
        let mut total_correct = 0;
        let mut total_partially_correct = 0;
        let mut total = 0;
        let mut aggregate_score = 0.0;

        let questions = repetition::choose_questions(&self.questions, &options);
        if questions.len() == 0 {
            return Err(QuizError::EmptyQuiz);
        }

        if let Some(instructions) = &self.instructions {
            ui.instructions(&instructions)?;
        }

        if questions.iter().any(|q| q.timed()) {
            ui.warning("This quiz contains timed questions!")?;
        }

        for q in questions.iter() {
            let result = q.ask(ui);
            if let Ok(result) = result {
                let score = result.score;
                results.push(result);

                total += 1;
                aggregate_score += score;
                if score == 1.0 {
                    total_correct += 1;
                } else if score > 0.0 {
                    total_partially_correct += 1;
                }
            } else if let Err(QuizError::ReadlineInterrupted) = result {
                break;
            } else if let Err(e) = result {
                return Err(e);
            }
        }

        let total_incorrect = total - total_correct - total_partially_correct;
        let score = (aggregate_score / (total as f64)) * 100.0;
        let ret = QuizResult {
            time_finished: chrono::Utc::now(),
            total,
            total_correct,
            total_partially_correct,
            total_incorrect,
            score,
            per_question: results,
        };
        ui.results(&ret)?;
        Ok(ret)
    }
}


pub trait Question: std::fmt::Debug {
    fn ask(&self, ui: &mut CmdUI) -> Result<QuestionResult>;
    fn get_common(&self) -> &QuestionCommon;
    fn get_text(&self) -> String;
    fn timed(&self) -> bool { false }
    fn flip(&mut self) {}
}


#[derive(Debug, Clone)]
pub struct QuestionCommon {
    pub id: String,
    pub prior_results: Vec<QuestionResult>,
    pub tags: Vec<String>,
    pub location: Option<Location>,
}


#[derive(Debug, Clone)]
pub struct ShortAnswerQuestion {
    pub text: String,
    pub answer: Answer,
    /// If specified, the number of seconds the user has to answer the question for full
    /// credit. Once passed, the user can still get partial credit up if she answers
    /// within `2*timeout` seconds.
    pub timeout: Option<u64>,
    pub common: QuestionCommon,
}


impl Question for ShortAnswerQuestion {
    fn ask(&self, ui: &mut CmdUI) -> Result<QuestionResult> {
        ui.text(&self.text)?;
        if let Some(guess) = ui.prompt()? {
            if check(&self.answer, &guess) {
                ui.correct()?;
                let elapsed = ui.get_elapsed();
                let (score, timed_out) = calculate_score(1.0, self.timeout, elapsed);
                ui.score(score, timed_out)?;
                Ok(mkresult(&self.get_common().id, Some(guess), score))
            } else {
                ui.incorrect(Some(&self.answer[0]))?;
                ui.score(0.0, false)?;
                Ok(mkresult(&self.get_common().id, Some(guess), 0.0))
            }
        } else {
            ui.incorrect(Some(&self.answer[0]))?;
            ui.score(0.0, false)?;
            Ok(mkresult(&self.get_common().id, None, 0.0))
        }
    }

    fn get_common(&self) -> &QuestionCommon { &self.common }
    fn get_text(&self) -> String { self.text.clone() }
    fn timed(&self) -> bool { self.timeout.is_some() }
}


#[derive(Debug, Clone)]
pub struct FlashcardQuestion {
    pub front: Answer,
    pub back: Answer,
    pub front_context: Option<String>,
    pub back_context: Option<String>,
    pub timeout: Option<u64>,
    pub common: QuestionCommon,
}


impl Question for FlashcardQuestion {
    fn ask(&self, ui: &mut CmdUI) -> Result<QuestionResult> {
        if let Some(context) = &self.front_context {
            ui.text(&format!("{} [{}]", self.front[0], context))?;
        } else {
            ui.text(&self.front[0])?;
        }

        if let Some(guess) = ui.prompt()? {
            if check(&self.back, &guess) {
                ui.correct()?;
                let elapsed = ui.get_elapsed();
                let (score, timed_out) = calculate_score(1.0, self.timeout, elapsed);
                ui.score(score, timed_out)?;
                Ok(mkresult(&self.get_common().id, Some(guess), score))
            } else {
                ui.incorrect(Some(&self.back[0]))?;
                ui.score(0.0, false)?;
                Ok(mkresult(&self.get_common().id, Some(guess), 0.0))
            }
        } else {
            ui.incorrect(Some(&self.back[0]))?;
            ui.score(0.0, false)?;
            Ok(mkresult(&self.get_common().id, None, 0.0))
        }
    }

    fn get_common(&self) -> &QuestionCommon { &self.common }
    fn get_text(&self) -> String { self.front[0].clone() }
    fn timed(&self) -> bool { self.timeout.is_some() }

    fn flip(&mut self) {
        mem::swap(&mut self.front, &mut self.back);
        mem::swap(&mut self.front_context, &mut self.back_context);
    }
}


#[derive(Debug, Clone)]
pub struct ListQuestion {
    pub text: String,
    pub answer_list: Vec<Answer>,
    pub no_credit: Vec<String>,
    pub common: QuestionCommon,
}


impl Question for ListQuestion {
    fn ask(&self, ui: &mut CmdUI) -> Result<QuestionResult> {
        let n = self.answer_list.len();
        // TODO: Replace with array?
        let mut satisfied = Vec::<bool>::with_capacity(n);
        for _ in 0..n {
            satisfied.push(false);
        }

        ui.text(&self.text)?;
        let mut count = 0;
        let mut responses = Vec::new();
        while count < n {
            if let Some(guess) = ui.prompt()? {
                responses.push(guess.clone());

                if let Some(index) = check_one(&self.answer_list, &guess) {
                    if satisfied[index] {
                        ui.repeat()?;
                    } else {
                        satisfied[index] = true;
                        ui.correct()?;
                        count += 1;
                    }
                } else {
                    if check(&self.no_credit, &guess) {
                        ui.no_credit()?;
                    } else {
                        ui.incorrect(None)?;
                        count += 1;
                    }
                }
            } else {
                ui.incorrect(None)?;
                break;
            }
        }

        let mut missed = Vec::new();
        for (i, b) in satisfied.iter().enumerate() {
            if !b {
                missed.push(self.answer_list[i][0].as_str());
            }
        }

        if missed.len() > 0 {
            ui.missed(&missed)?;
        }
        let score = (n - missed.len()) as f64 / (n as f64);
        ui.score(score, false)?;

        Ok(mkresultlist(&self.get_common().id, responses, score))
    }

    fn get_common(&self) -> &QuestionCommon { &self.common }
    fn get_text(&self) -> String { self.text.clone() }
}


#[derive(Debug, Clone)]
pub struct OrderedListQuestion {
    pub text: String,
    pub answer_list: Vec<Answer>,
    pub no_credit: Vec<String>,
    pub common: QuestionCommon,
}


impl Question for OrderedListQuestion {
    fn ask(&self, ui: &mut CmdUI) -> Result<QuestionResult> {
        ui.text(&self.text)?;

        let mut ncorrect = 0;
        let mut responses = Vec::new();
        for answer in self.answer_list.iter() {
            if let Some(guess) = ui.prompt()? {
                responses.push(guess.clone());

                if check(answer, &guess) {
                    ui.correct()?;
                    ncorrect += 1;
                } else {
                    ui.incorrect(Some(&answer[0]))?;
                }
            } else {
                ui.incorrect(Some(&answer[0]))?;
                break;
            }
        }
        let score = (ncorrect as f64) / (self.answer_list.len() as f64);
        ui.score(score, false)?;
        Ok(mkresultlist(&self.get_common().id, responses, score))
    }

    fn get_common(&self) -> &QuestionCommon { &self.common }
    fn get_text(&self) -> String { self.text.clone() }
}


#[derive(Debug, Clone)]
pub struct MultipleChoiceQuestion {
    pub text: String,
    pub answer: Answer,
    pub choices: Vec<String>,
    pub timeout: Option<u64>,
    pub common: QuestionCommon,
}


impl Question for MultipleChoiceQuestion {
    fn ask(&self, ui: &mut CmdUI) -> Result<QuestionResult> {
        ui.text(&self.text)?;

        let mut choices: Vec<&str> = self.choices.iter().map(|s| s.as_str()).collect();
        let mut rng = thread_rng();
        // Shuffle once so that we don't always pick the first three candidates listed.
        choices.shuffle(&mut rng);
        choices.truncate(3);

        let answer = self.answer.choose(&mut rng).unwrap();
        choices.push(&answer);
        // Shuffle again so that the position of the correct answer is random.
        choices.shuffle(&mut rng);

        ui.choices(&choices)?;
        let mut response = None;
        let mut correct = false;
        loop {
            if let Some(guess) = ui.prompt()? {
                if guess.len() != 1 {
                    continue;
                }

                let index = guess.to_ascii_lowercase().as_bytes()[0];
                if 97 <= index && index < 101 {
                    let guess = &self.choices[(index - 97) as usize];
                    response.replace(guess.clone());
                    if check(&self.answer, guess) {
                        ui.correct()?;
                        correct = true;
                    } else {
                        ui.incorrect(Some(&answer))?;
                    }
                    break;
                } else {
                    continue;
                }
            } else {
                ui.incorrect(Some(&answer))?;
            }
        }
        let (score, timed_out) = calculate_score(
            if correct { 1.0 } else { 0.0 }, self.timeout, ui.get_elapsed());
        ui.score(score, timed_out)?;
        Ok(mkresult(&self.get_common().id, response, score))
    }

    fn get_common(&self) -> &QuestionCommon { &self.common }
    fn get_text(&self) -> String { self.text.clone() }
    fn timed(&self) -> bool { self.timeout.is_some() }
}


/// Each member of the vector should be an equivalent answer, e.g.
/// `vec!["Mount Everest", "Everest"]`, not different answers to the same question. The
/// first element of the vector is taken to be the canonical form of the answer for
/// display.
pub type Answer = Vec<String>;


/// Represents the result of answering a question on a particular occasion.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuestionResult {
    #[serde(skip)]
    pub id: String,
    pub time_asked: chrono::DateTime<chrono::Utc>,
    /// If the question asked was a short answer question, then the user's response goes
    /// in this field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    /// If the question asked was a list question, then the user's responses go in this
    /// field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_list: Option<Vec<String>>,
    pub score: f64,

    // It would be convenient to include a reference to the `Question` object as a field
    // of this struct, but Rust's lifetimes make it more difficult than it's worth.
}


/// Represents the results of taking a quiz on a particular occasion.
#[derive(Debug)]
pub struct QuizResult {
    pub time_finished: chrono::DateTime<chrono::Utc>,
    pub total: usize,
    pub total_correct: usize,
    pub total_partially_correct: usize,
    pub total_incorrect: usize,
    pub score: f64,
    pub per_question: Vec<QuestionResult>,
}


/// Return `true` if `guess` matches any of the answers in `answer_list`.
pub fn check_any(answer_list: &Vec<Answer>, guess: &str) -> bool {
    for answer in answer_list.iter() {
        if check(answer, guess) {
            return true;
        }
    }
    false
}


/// Return the index of the first answer in `answer_list` that `guess` matches, or
/// `None` if `guess` satisfies none.
pub fn check_one(answer_list: &Vec<Answer>, guess: &str) -> Option<usize> {
    for (i, answer) in answer_list.iter().enumerate() {
        if check(answer, guess) {
            return Some(i);
        }
    }
    None
}


/// Return `true` if the given string is equivalent to the Answer object.
pub fn check(ans: &Answer, guess: &str) -> bool {
    for variant in ans.iter() {
        if normalize(&variant) == normalize(&guess) {
            return true;
        }
    }
    false
}


fn normalize(guess: &str) -> String {
    String::from(guess.to_lowercase())
}


/// Calculate the final score given the base score, the question's timeout, and how
/// long it took to answer the question. Return `(score, timed_out)` where `timed_out`
/// indicates whether the time limit was exceeded.
fn calculate_score(
    base_score: f64, timeout: Option<u64>, elapsed: time::Duration) -> (f64, bool) {
    if let Some(timeout) = timeout {
        let e = elapsed.as_millis() as i128;
        let t = (timeout * 1000) as i128;
        if e <= t {
            (base_score, false)
        } else if e < 2*t {
            ((base_score * (-1.0 / (t as f64)) * (e - 2 * t) as f64), true)
        } else {
            (0.0, true)
        }
    } else {
        (base_score, false)
    }
}


/// Construct a `QuestionResult` object.
fn mkresult(id: &str, response: Option<String>, score: f64) -> QuestionResult {
    QuestionResult {
        id: String::from(id),
        time_asked: chrono::Utc::now(),
        score,
        response,
        response_list: None,
    }
}


/// Construct a `QuestionResult` object with a list of responses.
fn mkresultlist(id: &str, responses: Vec<String>, score: f64) -> QuestionResult {
    QuestionResult {
        id: String::from(id),
        time_asked: chrono::Utc::now(),
        score,
        response: None,
        response_list: Some(responses),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checking_answers_works() {
        let ans = vec![s("Barack Obama"), s("Obama")];

        assert!(check(&ans, "Barack Obama"));
        assert!(check(&ans, "barack obama"));
        assert!(check(&ans, "Obama"));
        assert!(check(&ans, "obama"));
        assert!(!check(&ans, "Mitt Romney"));
    }

    fn s(mystr: &str) -> String {
        String::from(mystr)
    }
}
