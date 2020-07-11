use ncurses::*;
use serde::Deserialize;
// use std::thread::LocalKey;

#[derive(Debug)]
pub struct Question {
    pub greek: String,
    pub answers: Vec<AnswerOption>,
}

impl Clone for Question {
    fn clone(&self) -> Self {
        let greek = self.greek.clone();
        let mut answers: Vec<AnswerOption> = Vec::new();
        for answer in &self.answers {
            let ao_dup = answer.clone();
            answers.push(ao_dup);
        }
        Question {
            greek,
            answers,
        }
    }
}

#[derive(Debug)]
pub struct AnswerOption {
    pub mark: u8,
    pub answer: String,
    pub feedback: String,
}

impl Clone for AnswerOption {
    fn clone(&self) -> Self {
        let mark = self.mark;
        let answer = self.answer.clone();
        let feedback = self.feedback.clone();
        AnswerOption {
            mark,
            answer,
            feedback,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Record {
    pub greek: String,
    #[serde(rename = "Part of Speech")]
    pub part_of_speech: String,
    pub english: String,
}

pub fn clear_win(win: WINDOW) {
    wclear(win);
    box_(win, 0, 0);
    wrefresh(win);
}

pub fn wprint(win: WINDOW, s: &str) {
    for (num, line) in s.split("\n").enumerate() {
        mvwaddstr(win, 1 + num as i32, 1, line);
    }
    wrefresh(win);
}

pub fn overwrite_win(win: WINDOW, s: &str) {
    wclear(win);
    box_(win, 0, 0);
    for (num, line) in s.split("\n").enumerate() {
        mvwaddstr(win, 1 + num as i32, 1, line);
    }
    wrefresh(win);
}

pub fn prompt() {
    mvwaddstr(stdscr(), getmaxy(stdscr()) - 2, 30, "<--Press any key-->");
    refresh();
    getch();
}

