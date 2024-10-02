use chrono::{NaiveDateTime, TimeZone, Utc};
use core::panic;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

fn main() {
    let templates: Vec<Vec<String>> = read_lines("./templates.csv")
        .unwrap()
        .map_while(Result::ok)
        .map(|line| {
            let parts: Vec<String> = line
                .trim_end_matches(" ")
                .trim_start_matches("<*>")
                .trim_end_matches("<*>")
                .split("<*>")
                .map(|part| part.to_string())
                .collect();
            // println!("{:?}", parts);
            parts
        })
        .collect();

    let mut labels: HashMap<String, String> = HashMap::new();
    let mut lines = read_lines("labels.csv").unwrap();
    lines.next(); // skip the header

    for line in lines.map_while(Result::ok) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 2 {
            let name = parts[0].to_string();
            let label = parts[1].trim().to_string();
            labels.insert(name, label);
        }
    }

    println!("id;event_type;seq_id;time;label");
    if let Ok(lines) = read_lines("./sorted.log") {
        let mut prev_timestamp = Utc::now().timestamp();

        // Consumes the iterator, returns an (Optional) String
        for (line_i, line) in lines.map_while(Result::ok).enumerate() {
            // println!("{}", line);
            let line_id = line_i + 1;
            let mut template_id = 0;
            let mut positions = vec![];
            for (t_i, template) in templates.iter().enumerate() {
                let mut current_pos = 0;
                let mut found = true;
                for part in template {
                    match line.as_str()[current_pos..].find::<&str>(part) {
                        Some(pos) => {
                            current_pos += pos;
                            positions.push(current_pos);
                            current_pos += part.len();
                            positions.push(current_pos);
                        }
                        None => {
                            positions.clear();
                            found = false;
                            break;
                        }
                    }
                }
                if found {
                    // println!("{:?}", positions);
                    template_id = t_i + 1;
                    break;
                }
            }
            positions.insert(0, 0);
            positions.push(line.len());

            let mut blk_id: Option<&str> = None;
            let mut extra_blk_ids: Vec<&str> = vec![];
            if template_id == 30 {
                let mut ids = line[positions[4]..positions[5]].trim().split(" ");
                blk_id = Some(ids.next().expect(&line));
                for id in ids {
                    extra_blk_ids.push(id);
                }
            } else {
                let blk_end_patterns: &[_] = &[' ', '.', '\''];
                for (i, j) in (0..positions.len())
                    .step_by(2)
                    .zip((1..positions.len()).step_by(2))
                {
                    let param: &str = &line[positions[i]..positions[j]];
                    if let Some(p) = param.find("blk_") {
                        blk_id = Some(param[p..].split(blk_end_patterns).next().unwrap());
                    }
                }
            }

            //time_string = line[:13] # timestamp format in logs: 081111 111607
            // if time_string == "du: cannot ac": # last few lines in log file do not have a time stamp
            //      timestamp = prev_timestamp # assume that the lines without timestamp occur at the same time as the logs before
            // else:
            //      timestamp = datetime.datetime(year=int('20' + time_string[:2]), month=int(time_string[2:4]), day=int(time_string[4:6]), hour=int(time_string[7:9]), minute=int(time_string[9:11]), second=int(time_string[11:13])).replace(tzinfo=timezone.utc).timestamp()
            //      prev_timestamp = timestamp

            let time_string = &line[..13]; // timestamp format in logs: 081111 111607

            let timestamp = if time_string == "du: cannot ac" {
                // last few lines in log file do not have a timestamp
                prev_timestamp // assume that the lines without timestamp occur at the same time as the logs before
            } else {
                let datetime_str = format!(
                    "20{}-{}-{} {}:{}:{}",
                    &time_string[0..2],   // year
                    &time_string[2..4],   // month
                    &time_string[4..6],   // day
                    &time_string[7..9],   // hour
                    &time_string[9..11],  // minute
                    &time_string[11..13]  // second
                );
                let naive_datetime =
                    NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S").unwrap();
                let datetime = Utc.from_utc_datetime(&naive_datetime).timestamp();
                prev_timestamp = datetime;
                datetime
            };

            if let Some(label) = labels.get(blk_id.unwrap()) {
                println!(
                    "{};{};{};{:.1};{}",
                    line_id,
                    template_id,
                    blk_id.unwrap(),
                    timestamp as f64,
                    label
                );
            } else {
                panic!(
                    "Label not found for blk_id: {}, line_id: {}, template_id: {}",
                    blk_id.unwrap(),
                    line_id,
                    template_id
                );
            }
            for extra_blk_id in extra_blk_ids.iter() {
                if let Some(label) = labels.get::<str>(extra_blk_id) {
                    println!(
                        "{};{};{};{:.1};{}",
                        line_id, template_id, extra_blk_id, timestamp as f64, label
                    );
                } else {
                    panic!(
                        "Label not found for extra_blk_id: {}, line_id: {}, template_id: {}",
                        extra_blk_id, line_id, template_id
                    );
                }
            }
        }
    }
}

// The output is wrapped in a Result to allow matching on errors.
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
