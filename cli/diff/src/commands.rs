use std::{cmp::max, env, format, fs, io};

pub enum Line {
    Same(usize),
    Removed(usize),
    Added(usize),
    Changed(usize),
}

pub enum Outcome {
    Same,
    Changed,
    // NotFound,
}

pub fn run_command() -> Result<Outcome, String> {
    let args: Vec<String> = env::args().collect();

    let file_path_1 = args
        .get(1)
        .ok_or_else(|| format!("File path 1 not found!"))?;
    let full_path_1 = format!("{}/src{file_path_1}", env!("CARGO_MANIFEST_DIR"));

    let file_path_2 = args
        .get(2)
        .ok_or_else(|| format!("File path 2 not found!"))?;
    let full_path_2 = format!("{}/src{file_path_2}", env!("CARGO_MANIFEST_DIR"));

    let (file_1, file_2) = get_files(&full_path_1, &full_path_2).map_err(|e| e.to_string())?;

    let (_, outcome) = compare_files(&file_1, &file_2);

    Ok(outcome)
}

fn get_files(path_1: &str, path_2: &str) -> io::Result<(String, String)> {
    let file_1 = fs::read_to_string(path_1)?;
    let file_2 = fs::read_to_string(path_2)?;

    Ok((file_1, file_2))
}

fn compare_files(file_1: &str, file_2: &str) -> (Vec<Line>, Outcome) {
    let file_1_vec: Vec<&str> = file_1.lines().collect();
    let file_2_vec: Vec<&str> = file_2.lines().collect();
    let idx = max(file_1_vec.len(), file_2_vec.len());
    let mut diffs: Vec<Line> = Vec::with_capacity(idx);
    let mut outcome = Outcome::Same;

    for i in 0..idx {
        let line_1;
        let line_2;

        match file_1_vec.get(i) {
            Some(t) => line_1 = t,
            None => {
                diffs.push(Line::Added(i));
                outcome = Outcome::Changed;
                continue;
            }
        }

        match file_2_vec.get(i) {
            Some(t) => line_2 = t,
            None => {
                diffs.push(Line::Removed(i));
                outcome = Outcome::Changed;
                continue;
            }
        }

        if line_1 == line_2 {
            diffs.push(Line::Same(i));
        } else {
            diffs.push(Line::Changed(i));
            outcome = Outcome::Changed;
        }
    }

    (diffs, outcome)
}

// fn show_result(mut diffs: Vec<Line>, file: &str) -> Outcome {
//     for i in file.lines() {

//     }
// }

//handle the output
//print file 2, paint the lines based on the diffs array??

//get files (check if this is efficient enough or use buffer)
//read the files
//compare lines from file 1(older) to 2(newer)
// return response (have enum representing it)
//determing what to show in main

// impl From<io::Error> for String {
//     fn from(value: io::Error) -> Self {
//         format!()
//     }
// }
