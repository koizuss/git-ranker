use regex::Regex;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

type Scores = Arc<Mutex<HashMap<String, u32>>>;

async fn get_git_version() -> String {
    let output = Command::new("git")
        .arg("--version")
        .output()
        .await
        .expect("`git` not found");

    let stdout = String::from_utf8_lossy(&output.stdout);

    return Regex::new(r"\d+\.\d+\.\d+")
        .unwrap()
        .captures(&stdout)
        .unwrap()
        .get(0)
        .unwrap()
        .as_str()
        .to_string();
}

#[tokio::main]
async fn main() {
    let _git_version = get_git_version().await;
    // println!("{}", _git_version);

    let scores: Scores = Arc::new(Mutex::new(HashMap::new()));

    let mut git_grep = Command::new("git")
        .arg("grep")
        .arg("-Il")
        .arg("\\'\\'")
        .stdout(Stdio::piped())
        .spawn()
        .expect("`git grep` failed");

    let mut git_grep_lines = BufReader::new(git_grep.stdout.take().unwrap()).lines();
    while let Some(file) = git_grep_lines.next_line().await.unwrap() {
        // TODO: exclude
        // println!(">> {}", file);

        let mut git_blame = Command::new("git")
            .arg("blame")
            .arg(file)
            .stdout(Stdio::piped())
            .spawn()
            .expect("`git blame` failed"); // HACK: どのファイルか出す？

        let mut git_blame_lines = BufReader::new(git_blame.stdout.take().unwrap()).lines();
        while let Some(line) = git_blame_lines.next_line().await.unwrap() {
            let author = Regex::new(r"^[^\(]*\((.*) \d{4,4}-\d{2,2}-\d{2,2}[^a-zA-Z]+\).*$")
                .unwrap()
                .captures(&line)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
                .trim();

            // println!("{}", author);

            let mut locked_scores = scores.lock().unwrap();
            let value_by_auth = *locked_scores.get(author).unwrap_or(&(0 as u32)) + 1;
            let value_total = *locked_scores.get("total").unwrap_or(&(0 as u32)) + 1;
            locked_scores.insert(author.to_string(), value_by_auth);
            locked_scores.insert("total".to_string(), value_total);
        }
    }

    let mut locked_scores = scores.lock().unwrap();
    let total = *locked_scores.get("total").unwrap_or(&(0 as u32));
    locked_scores.remove("total");

    let iter = locked_scores.iter();
    for (author, value) in iter {
        let ratio = ((*value as f64) / (total as f64) * 10000.0).round() / 100.0;
        println!("{}\t{}\t{}%", author, value, ratio);
    }
}
