use csv::Reader;
// use english_past::{lookup, Verb};
use greek_vocab_test_maker::*;
use mw_past::{lookup, Verb};
use ncurses::*;
use regex::Regex;
use std::char;
use std::error::Error;
use std::fs::read_dir;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::thread;

// declare windows as global variables
thread_local!(
    pub static HEADER: WINDOW = newwin(5, 80, 0, 0);
    pub static MAIN_WIN: WINDOW = newwin(getmaxy(stdscr()) - 10, 80, 5, 0);
    pub static KEYS_WIN: WINDOW = newwin(3, 80, getmaxy(stdscr()) - 5, 0);
    pub static INPUT_WIN: WINDOW = newwin(2, 80, getmaxy(stdscr()) - 2, 0);
);

fn main() {
    if let Err(e) = run() {
        println!("Application error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut questions: Vec<Question> = Vec::new();
    setlocale(ncurses::constants::LcCategory::all, "utf8");
    initscr();
    noecho();
    loop {
        // clear the screen
        clear();
        // get dimensions
        let mut max_y = 0;
        let mut max_x = 0;
        getmaxyx(stdscr(), &mut max_y, &mut max_x);
        // do an initial refresh...or it won't let me do anything!
        refresh();
        // header window
        curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        // let header_win: WINDOW = newwin(5, 80, 0, 0);
        HEADER.with(|header| {
            overwrite_win(*header, "Greek vocab test for Moodle Maker");
            mvwaddstr(
                *header,
                3,
                1,
                &format!("Question count: {}", questions.len()),
            );
            wrefresh(*header);
        });

        MAIN_WIN.with(|main_win| {
            overwrite_win(
                *main_win,
                "Main menu:\n\n\
                           Select an option from the list below.\n\n\
                           - Add: Enter a question manually. Good luck!\n\n\
                           - Import: Create questions from words in a file.",
            );
        });

        // keys_win
        KEYS_WIN.with(|keys_win| {
            wclear(*keys_win);
            box_(*keys_win, 0, 0);
            mvwaddstr(
                *keys_win,
                1,
                1,
                "a: add    i: import    r: review    x: export    s: start again    q: quit",
            );
            wrefresh(*keys_win);
        });

        match char::from_u32(getch() as u32).unwrap() {
            'a' => unimplemented!(),
            'i' => import(&mut questions),
            'r' => pager(&mut questions),
            'x' => export(&questions),
            's' => questions.clear(),
            'q' => break,
            _ => continue,
        }
    }
    endwin();
    Ok(())
}

fn export(questions: &Vec<Question>) {
    // Put opening statement in xml file
    // fields are stage_number, folder_name
    macro_rules! xml_start {
        ($arg1:expr) => {
            format!(
                "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
             <quiz>\n\
             <!-- question: 0  -->\n\
             <question type=\"category\">\n\
             <category>\n\
             <text>$course$/{}</text>\n\
             </category>\n\
             </question>\n",
                $arg1
            )
            .as_bytes()
        };
    }

    // fields are question_number, question_name, question, questioncode
    macro_rules! xml_question {
        ($q_num:expr, $q_name:expr, $question:expr, $q_code:expr) => {
            format!(
                "<!-- question: {}  -->\n\
                 <question type=\"cloze\" > \n\
                 <name>\n\
                 <text>{}</text>\n\
                 </name>\n\
                 <questiontext>\n\
                 <text>\n\
                 <![CDATA[<p>{}</p>\n\
                 <p><font size=\"4\" face=\"times new roman,times,serif\">{}.</font></p>]]>\n\
                 </text>\n\
                 </questiontext>\n\
                 <generalfeedback>\n\
                 <text></text>\n\
                 </generalfeedback>\n\
                 <shuffleanswers>0</shuffleanswers>\n\
                 </question>\n",
                $q_num, $q_name, $question, $q_code
            ).as_bytes()
        }
    }

    let xml_end: String = String::from("</quiz>\n");

    let stage_number = get_input_with_initial(
        "Enter a test no. or modify folder path: ",
        "top/Vocabulary/A_",
    );
    let ex_name = get_input_with_initial(
        "Enter a basename for questions: ",
        &stage_number.replace("top/Vocabulary/", ""),
    );

    // set up Writer
    let f = File::create("./upload.xml").expect("Unable to create file");
    let mut writer = BufWriter::new(f);

    // make initial write
    writer
        .write(xml_start!(stage_number))
        .expect("Unable to write xml start.");

    let mut question_number: u32 = 1000;
    let mut question_name: String;
    let mut question_word: &str;
    let mut question_code = String::new();
    //Now loop over question data
    for question in questions.iter() {
        question_number += 1;
        question_name = format!("{}_q_{}", ex_name, question_number);
        question_word = question.greek.as_str();
        // question_code = moodle_shortanswer(&question.answers[num]);
        question_code.clear();
        question_code = format!(
            "{} {}",
            question_code,
            moodle_shortanswer(&question.answers)
        );
        writer
            .write(xml_question!(
                question_number,
                question_name,
                question_word,
                question_code
            ))
            .expect("Unable to write xml question.");
    }
    writer
        .write(xml_end.as_bytes())
        .expect("Unable to write xml end.");
    writer.flush().expect("Unable to write data.");
    // display message to user
    INPUT_WIN.with(|input_win| {
        wclear(*input_win);
        mvwaddstr(*input_win, 0, 0, "upload.xml written!");
        wrefresh(*input_win);
    });
    getch();
}

fn moodle_shortanswer(answers: &Vec<AnswerOption>) -> String {
    let mut question_string = String::new();
    for answer in answers {
        question_string = format!(
            "{}~%{}%{}#{}",
            question_string, answer.mark, answer.answer, answer.feedback
        );
    }
    format!("{{1:SHORTANSWER:{}}}", question_string)
}

fn import(questions: &mut Vec<Question>) {
    let file: PathBuf = get_file();
    let mut rdr = Reader::from_path(file).unwrap();
    INPUT_WIN.with(|input_win| {
        wclear(*input_win);
        mvwaddstr(*input_win, 0, 0, "Loading questions:");
        wrefresh(*input_win);
        wmove(*input_win, 1, 0);
    });
    let (tx, rx) = mpsc::channel();
    for result in rdr.deserialize() {
        let record: Record = result.unwrap();
        match &record.part_of_speech {
            pos if pos.contains("verb") => {
                let tx1 = mpsc::Sender::clone(&tx);
                thread::spawn(move || {
                    let verb_questions: Vec<Question> = build_verb(record.greek, record.english);
                    tx1.send(verb_questions).unwrap();
                });
            }
            _ => {
                build_non_verb(questions, record.greek, record.english);
                progress_bar();
            }
        }
    }
    drop(tx);
    for verb_questions in &rx {
        for verb_question in verb_questions {
            questions.push(verb_question);
            progress_bar();
        }
    }
}

fn progress_bar() {
    INPUT_WIN.with(|input_win| {
        waddstr(*input_win, ".");
        wrefresh(*input_win);
    })
}

fn get_file() -> PathBuf {
    let mut entries = list_files().unwrap();
    entries.sort_unstable();
    let mut choice: usize = 0;
    MAIN_WIN.with(|main_win| {
        overwrite_win(*main_win, "Select file: ");
        for (num, entry) in entries.iter().enumerate() {
            mvwaddstr(
                *main_win,
                2 + num as i32,
                1,
                &format!("{}: {:?}", num + 1, entry),
            );
        }
        wrefresh(*main_win);
        choice = get_num_input(0, entries.len());
        clear_win(*main_win);
        wrefresh(*main_win);
        // prompt();
    });
    entries[choice].to_owned()
}

fn list_files() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let entries = read_dir(".")?;
    let mut list: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let file = entry?.path();
        if file.is_file() {
            list.push(file);
        }
    }
    Ok(list)
}

fn clean_text(s: &str) -> String {
    let re1 = Regex::new(r"\(.*\)").unwrap();
    let re2 = Regex::new(r"\s*GEN").unwrap();
    let re3 = Regex::new(r"\s*ACC").unwrap();
    let re4 = Regex::new(r"\s*DAT").unwrap();
    let re5 = Regex::new(r";").unwrap();
    let re6 = Regex::new(r"/").unwrap();
    let s = re1.replace_all(&s, "");
    let s = re2.replace_all(&s, "");
    let s = re3.replace_all(&s, "");
    let s = re4.replace_all(&s, "");
    let s = re5.replace_all(&s, ",");
    let s = re6.replace_all(&s, ",");
    format!("{}", s)
}

fn build_non_verb(questions: &mut Vec<Question>, greek: String, english: String) {
    let mut answer_options: Vec<AnswerOption> = Vec::new();
    let english = clean_text(&english);
    for answer in english.split(",").collect::<Vec<&str>>() {
        let answer_option = AnswerOption {
            mark: 100,
            answer: answer.trim().to_string(),
            feedback: "Well done!".to_string(),
        };
        answer_options.push(answer_option);
    }
    let question = Question {
        greek: greek,
        answers: answer_options,
    };
    questions.push(question);
}

fn build_verb(greek: String, english: String) -> Vec<Question> {
    // println!("Greek verb: {}", greek);
    // println!("English: {}", english);
    let mut questions: Vec<Question> = Vec::new();
    let english = english.replace("I ", "");
    let english = clean_text(&english);
    let answers = english.split(",").collect::<Vec<&str>>();
    let mut verb_collection: Vec<Verb> = Vec::new();
    for answer in &answers {
        let verb = lookup(answer.trim());
        verb_collection.push(verb);
    }
    // println!("Answers: {:?}", answers);
    let greek_parts = greek.split(",").collect::<Vec<&str>>();
    // println!("Parts: {:?}", greek_parts);
    //present tense
    let mut answer_options: Vec<AnswerOption> = Vec::new();
    for verb in &verb_collection {
        // println!("Present: {}", answer.trim());
        let answer_option = AnswerOption {
            mark: 100,
            answer: format!("I {}", verb.present),
            feedback: "Well done!".to_string(),
        };
        answer_options.push(answer_option);
        // prepare partially correct answer with wildcards
        let answer_option_close = AnswerOption {
            mark: 50,
            answer: verb.asterisked.clone(),
            feedback: "Close! What part of the verb is this?".to_string(),
        };
        answer_options.push(answer_option_close);
    }
    let present = Question {
        greek: greek_parts[0].trim().to_string(),
        answers: answer_options,
    };
    questions.push(present);
    // future tense
    if greek_parts.len() > 1 {
        let mut answer_options: Vec<AnswerOption> = Vec::new();
        for verb in &verb_collection {
            let answer_option = AnswerOption {
                mark: 100,
                answer: format!("I*ll {}", verb.present),
                feedback: "Well done!".to_string(),
            };
            answer_options.push(answer_option);
            // prepare partially correct answer with wildcards
            let answer_option_close = AnswerOption {
                mark: 50,
                answer: verb.asterisked.clone(),
                feedback: "Close! What part of the verb is this?".to_string(),
            };
            answer_options.push(answer_option_close);
        }
        let future = Question {
            greek: greek_parts[1].trim().to_string(),
            answers: answer_options,
        };
        questions.push(future);
    }
    // aorist tense
    if greek_parts.len() > 2 {
        let mut answer_options: Vec<AnswerOption> = Vec::new();
        for verb in &verb_collection {
            let answer_option = AnswerOption {
                mark: 100,
                answer: format!("I {}", verb.past_simple),
                feedback: "Well done!".to_string(),
            };
            answer_options.push(answer_option);
            // prepare partially correct answer with wildcards
            let answer_option_close = AnswerOption {
                mark: 50,
                answer: verb.asterisked.clone(),
                feedback: "Close! What part of the verb is this?".to_string(),
            };
            answer_options.push(answer_option_close);
        }
        let aorist = Question {
            greek: greek_parts[2].trim().to_string(),
            answers: answer_options,
        };
        questions.push(aorist);
    }
    // aorist passive
    if greek_parts.len() > 3 {
        let mut answer_options: Vec<AnswerOption> = Vec::new();
        for verb in &verb_collection {
            let answer_option = AnswerOption {
                mark: 100,
                answer: format!("I was {}", verb.past_part),
                feedback: "Well done!".to_string(),
            };
            answer_options.push(answer_option);
            // prepare partially correct answer with wildcards
            let answer_option_close = AnswerOption {
                mark: 50,
                answer: verb.asterisked.clone(),
                feedback: "Close! What part of the verb is this?".to_string(),
            };
            answer_options.push(answer_option_close);
        }
        let aorist_passive = Question {
            greek: greek_parts[3].trim().to_string(),
            answers: answer_options,
        };
        questions.push(aorist_passive);
    }
    questions
}

fn pager(questions: &mut Vec<Question>) {
    // update keys_win
    fn pager_keys() {
        KEYS_WIN.with(|keys_win| {
            overwrite_win(
                *keys_win,
                "a: add    c: duplicate    d: delete    e: edit    q: quit",
            );
            wrefresh(*keys_win);
        });
    }
    pager_keys();
    // work out how much space there is to display questions
    MAIN_WIN.with(|main_win| {
        let max_lines: i32 = getmaxy(*main_win);
        let mut line_count: i32 = 1;
        let mut current_q: usize = 0;
        let mut cached_q: usize = 0;
        let mut next_cached_q: usize = 0;
        let mut lines_required: usize = 0;
        // iterate over questions and pause at the end of the window
        loop {
            let max_qs: usize = questions.len();
            if line_count == 1 {
                next_cached_q = current_q;
            }
            if current_q < max_qs {
                lines_required = questions[current_q].answers.len();
            }
            if current_q < max_qs && lines_required < (max_lines - (line_count + 3)) as usize {
                mvwaddstr(
                    *main_win,
                    line_count,
                    1,
                    &format!("{}: {}", current_q + 1, questions[current_q].greek),
                );
                // mvwin(*main_win, line_count, 1);
                // print!("{}: {}", current_q + 1, questions[current_q].greek);
                line_count += 1;
                for answer in &questions[current_q].answers {
                    let q_str: &str = &format!(
                        "{:5} | {:30}| {:28}",
                        answer.mark, answer.answer, answer.feedback
                    );
                    mvwaddstr(*main_win, line_count, 1, q_str);
                    line_count += 1;
                }
                current_q += 1;
                wrefresh(*main_win);
            } else {
                mvwaddstr(
                    *main_win,
                    max_lines - 2,
                    1,
                    "<---| n: next page | p: previous page | s: start again |--->",
                );
                wrefresh(*main_win);
                match char::from_u32(getch() as u32).unwrap() {
                    // match get_input(">> ").as_str() {
                    'n' => {
                        wclear(*main_win);
                        box_(*main_win, 0, 0);
                        cached_q = next_cached_q;
                        line_count = 1;
                    }
                    'p' => {
                        wclear(*main_win);
                        box_(*main_win, 0, 0);
                        wrefresh(*main_win);
                        current_q = cached_q;
                        line_count = 1;
                    }
                    's' => {
                        wclear(*main_win);
                        box_(*main_win, 0, 0);
                        wrefresh(*main_win);
                        current_q = 0;
                        line_count = 1;
                    }
                    'e' => {
                        edit(questions);
                        wclear(*main_win);
                        box_(*main_win, 0, 0);
                        wrefresh(*main_win);
                        current_q = next_cached_q;
                        line_count = 1;
                        pager_keys();
                    }
                    'c' => {
                        duplicate(questions);
                        wclear(*main_win);
                        box_(*main_win, 0, 0);
                        wrefresh(*main_win);
                        current_q = next_cached_q;
                        line_count = 1;
                        HEADER.with(|header| {
                            mvwaddstr(
                                *header,
                                3,
                                1,
                                &format!("Question count: {}", questions.len()),
                            );
                            wrefresh(*header);
                        });
                    }
                    'd' => {
                        remove_question(questions);
                        wclear(*main_win);
                        box_(*main_win, 0, 0);
                        wrefresh(*main_win);
                        current_q = next_cached_q;
                        line_count = 1;
                        HEADER.with(|header| {
                            mvwaddstr(
                                *header,
                                3,
                                1,
                                &format!("Question count: {}", questions.len()),
                            );
                            wrefresh(*header);
                        });
                    }
                    'q' => break,
                    _ => continue,
                }
            }
        }
        // get user input
        // options
        // show the next n questions
        // duplicate
        // delete
        // edit
        // quit and return to the main menu
    });
}

fn remove_question(questions: &mut Vec<Question>) {
    let choice = get_num_input(0, questions.len());
    questions.remove(choice);
}

fn duplicate(questions: &mut Vec<Question>) {
    let choice = get_num_input(0, questions.len());
    let question = &mut questions[choice];
    let dup = question.clone();
    questions.insert(choice, dup);
}

fn edit(questions: &mut Vec<Question>) {
    let choice = get_num_input(0, questions.len());
    let question = &mut questions[choice];
    loop {
        MAIN_WIN.with(|main_win| {
            let mut line_count: i32 = 1;
            overwrite_win(*main_win, &format!("1: {}", question.greek));
            line_count += 1;
            for (num, answer) in question.answers.iter().enumerate() {
                let q_str: &str = &format!(
                    "{}: |{:5} | {:35}| {:28}",
                    num + 2,
                    answer.mark,
                    answer.answer,
                    answer.feedback
                );
                mvwaddstr(*main_win, line_count, 1, q_str);
                line_count += 1;
            }
            wrefresh(*main_win);
        });
        KEYS_WIN.with(|keys_win| {
            overwrite_win(
                *keys_win,
                "e: edit    a: add    d: delete    r: reorder    s: substitute    b: back",
            );
        });
        // match get_input(">> ").as_str() {
        match char::from_u32(getch() as u32).unwrap() {
            'e' => edit_answer(question),
            'a' => add_answer(question),
            'd' => delete_answer(question),
            'r' => reorder_answers(question),
            's' => subs_answers(question),
            'b' => break,
            _ => continue,
        }
    }
}

fn reorder_answers(question: &mut Question) {
    // Get the answer
    let mut choice = get_num_input(1, question.answers.len());
    KEYS_WIN.with(|keys_win| {
        overwrite_win(*keys_win, "u: move up    d: move down    a: accept");
    });
    loop {
        MAIN_WIN.with(|main_win| {
            let mut line_count: i32 = 1;
            overwrite_win(*main_win, &format!("1: {}", question.greek));
            line_count += 1;
            for (num, answer) in question.answers.iter().enumerate() {
                let q_str: &str = &format!(
                    "{}: |{:5} | {:35}| {:28}",
                    num + 2,
                    answer.mark,
                    answer.answer,
                    answer.feedback
                );
                mvwaddstr(*main_win, line_count, 1, q_str);
                line_count += 1;
            }
            wrefresh(*main_win);
        });
        noecho();
        match char::from_u32(getch() as u32).unwrap() {
            'u' => {
                if choice > 1 {
                    question.answers[choice - 2..choice].rotate_right(1);
                    choice -= 1;
                }
            }
            'd' => {
                if choice < question.answers.len() {
                    question.answers[choice - 1..choice + 1].rotate_left(1);
                    choice += 1;
                }
            }
            'a' => break,
            _ => continue,
        }
        echo();
    }

    KEYS_WIN.with(|keys_win| {
        overwrite_win(
            *keys_win,
            "e: edit    a: add    d: delete    r: reorder    s: substitute    b: back",
        );
    });
}

fn edit_answer(question: &mut Question) {
    // Get the answer
    let mut choice = get_num_input(0, question.answers.len());
    if choice == 0 {
        let greek = &mut question.greek;
        *greek = get_input_with_initial("Edit the question word: ", greek);
    } else {
        choice -= 1;
        let answer = &mut question.answers[choice].answer;
        *answer = get_input_with_initial("Edit your answer: ", answer);
        // Get the mark
        let mark = &mut question.answers[choice].mark;
        let new_value = get_mark();
        if new_value > 0 {
            *mark = get_mark();
        }
        // Get feedback
        let feedback = &mut question.answers[choice].feedback;
        let new_fdbk = get_feedback();
        if new_fdbk.as_str() != "XXXX" {
            *feedback = get_feedback();
        }
    }
}

fn add_answer(question: &mut Question) {
    // Get the answer
    let answer = get_input("Enter an answer: ");
    // Get the mark
    let mark = get_mark();
    // Get feedback
    let feedback = get_feedback();
    let answeroption = AnswerOption {
        mark: mark,
        answer: answer,
        feedback: feedback,
    };
    question.answers.push(answeroption);
}

fn get_mark() -> u8 {
    MAIN_WIN.with(|main_win| {
        overwrite_win(
            *main_win,
            "Choose a mark.\n\
             1. 100%\n\
             2. 50%\n\
             3. 0%",
        )
    });
    match get_input("Which mark? ").as_str() {
        "1" => return 100,
        "2" => return 50,
        _ => return 0,
    };
}

fn get_feedback() -> String {
    MAIN_WIN.with(|main_win| {
        overwrite_win(
            *main_win,
            "Choose feedback.\n\
             1. Well done!\n\
             2. Close! What tense is this?\n\
             3. Input something else.",
        )
    });
    match get_input("Which feedback? ").as_str() {
        "1" => return "Well done!".to_string(),
        "2" => return "Close! What tense is this?".to_string(),
        "3" => return get_input("Enter your feedback: "),
        _ => return "XXXX".to_string(),
    };
}

fn delete_answer(question: &mut Question) {
    let choice = get_num_input(0, question.answers.len()) - 1;
    question.answers.remove(choice);
}

fn subs_answers(question: &mut Question) {
    let findtext = get_input("Enter text to replace: ");
    let replacement = get_input("Enter replacement text: ");
    for answer in question.answers.iter_mut() {
        let ans = &mut answer.answer;
        *ans = ans.replace(findtext.as_str(), replacement.as_str());
    }
}

pub fn get_input(prompt: &str) -> String {
    INPUT_WIN.with(|input_win| {
        wclear(*input_win);
        wrefresh(*input_win);
        wmove(*input_win, 0, 0);
    });
    curs_set(CURSOR_VISIBILITY::CURSOR_VISIBLE);
    let mut rl = rustyline::Editor::<()>::new();
    let input = rl.readline(prompt).unwrap();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    input
}

pub fn get_input_with_initial(prompt: &str, initial: &str) -> String {
    INPUT_WIN.with(|input_win| {
        wclear(*input_win);
        wrefresh(*input_win);
        wmove(*input_win, 0, 0);
    });
    curs_set(CURSOR_VISIBILITY::CURSOR_VISIBLE);
    let mut rl = rustyline::Editor::<()>::new();
    let input = rl.readline_with_initial(prompt, (initial, "")).unwrap();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    input
}

pub fn get_num_input(min: usize, max: usize) -> usize {
    // returns the index for a choice, i.e. choice - 1
    loop {
        INPUT_WIN.with(|input_win| {
            wclear(*input_win);
            wrefresh(*input_win);
            wmove(*input_win, 0, 0);
        });
        curs_set(CURSOR_VISIBILITY::CURSOR_VISIBLE);
        let mut rl = rustyline::Editor::<()>::new();
        let input = rl.readline("Enter a number: ").unwrap();
        if let Ok(num) = input.parse::<usize>() {
            if num > min && num < max + 2 {
                curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
                return num - 1;
            } else {
                INPUT_WIN.with(|input_win| {
                    // input_win is not boxed so can't call overwrite_win()
                    wclear(*input_win);
                    mvwaddstr(
                        *input_win,
                        0,
                        0,
                        "That number was not within the correct range. Try again!",
                    );
                    wrefresh(*input_win);
                    getch();
                });
            }
        }
    }
}
