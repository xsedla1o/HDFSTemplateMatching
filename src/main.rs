use aho_corasick::{AhoCorasick, MatchKind};
use atoi::FromRadix10;
use chrono::NaiveDate;
use core::str;
use std::collections::HashMap;
use std::fs::File;
use std::io::Error;
use std::io::{self, BufRead, BufWriter, Write};
use std::iter;
use std::path::Path;

fn find_subarray<T: PartialEq>(haystack: &[T], needle: &[T]) -> Option<usize> {
    find_subarray_from(haystack, needle, 0)
}

fn find_subarray_from<T: PartialEq>(
    haystack: &[T],
    needle: &[T],
    start_pos: usize,
) -> Option<usize> {
    let mut i = start_pos;
    while i + needle.len() <= haystack.len() {
        let mut found = true;
        for j in 0..needle.len() {
            if haystack[i + j] != needle[j] {
                found = false;
                break;
            }
        }
        if found {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn blk_to_i<T>(s: &[u8]) -> T
where
    T: FromRadix10 + std::ops::Neg<Output = T>,
{
    if s[4] == b'-' {
        let (num, _endbyte) = T::from_radix_10(&s[5..]);
        return -num;
    }
    let (num, _endbyte) = T::from_radix_10(&s[4..]);
    num
}

fn decode_label(label: &u8) -> &'static [u8] {
    match label {
        0 => b"Normal",
        1 => b"Anomaly",
        _ => panic!("Unknown label"),
    }
}

fn write_output(
    w: &mut BufWriter<File>,
    line_id: usize,
    template_id: usize,
    blk_id: &[u8],
    timestamp: i64,
    label: &u8,
) -> Result<(), Error> {
    let out_line = format!("{};{};", line_id, template_id);
    w.write_all(out_line.as_bytes())?;
    w.write_all(blk_id)?;
    let out_line = format!(";{}.0;", timestamp);
    w.write_all(out_line.as_bytes())?;
    w.write_all(decode_label(label))?;
    w.write_all(b"\n")?;
    Ok(())
}

fn main() -> Result<(), Error> {
    // Read templates
    let templates: Vec<Vec<Vec<u8>>> = read_lines("./templates.csv")
        .unwrap()
        .map_while(Result::ok)
        .map(|line| {
            let line_str = String::from_utf8(line).unwrap();
            let parts: Vec<Vec<u8>> = line_str
                .trim_end_matches(" ")
                .trim_start_matches("<*>")
                .trim_end_matches("<*>")
                .split("<*>")
                .map(|part| part.as_bytes().to_owned())
                .collect();
            parts
        })
        .collect();

    // Add frequency counter to templates for heuristic sorting
    let mut template_freqs: Vec<u64> = vec![0; templates.len()];

    // Create Aho-Corasick automaton for the first part of each template
    let patterns: Vec<&[u8]> = templates.iter().map(|t| t[0].as_slice()).collect();
    let mut unique_patterns = Vec::with_capacity(patterns.len());
    // Vec of Vecs to store the IDs of the templates that have the same pattern
    let mut unique_patterns_ids: Vec<Vec<usize>> = Vec::with_capacity(patterns.len());
    for (i, pat) in patterns.iter().enumerate() {
        let trimmed = pat.trim_ascii();
        if !unique_patterns.contains(&trimmed) {
            unique_patterns.push(trimmed);
            unique_patterns_ids.push(vec![i]);
        } else {
            let id = unique_patterns.iter().position(|&p| p == trimmed).unwrap();
            unique_patterns_ids[id].push(i);
        }
    }
    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .build(unique_patterns)
        .unwrap();

    // Read labels
    let mut labels: HashMap<i64, u8> = HashMap::new();
    let mut lines = read_lines("labels.csv")?;
    lines.next(); // skip the header

    for line in lines.map_while(Result::ok) {
        let parts: Vec<&[u8]> = line.split(|&b| b == b',').collect();
        if parts.len() >= 2 {
            let blk_num = blk_to_i(parts[0]);
            let label = match parts[1][0] {
                b'N' => 0,
                b'A' => 1,
                _ => panic!("Unknown label"),
            };
            labels.insert(blk_num, label);
        }
    }

    let mut sort_treshold = 100000;

    let mut w = BufWriter::new(File::create("parsed_rust.csv")?);
    w.write_all(b"id;event_type;seq_id;time;label\n")?;

    if let Ok(lines) = read_lines("./sorted.log") {
        let mut prev_timestamp = 0;

        let mut positions = Vec::with_capacity(10);
        let mut tried = Vec::with_capacity(templates.len());

        for (line_i, line) in lines.map_while(Result::ok).enumerate() {
            let line_id = line_i + 1;
            let mut template_id = 0;

            positions.clear();
            tried.clear();

            if line_id == sort_treshold {
                for ids in unique_patterns_ids.iter_mut() {
                    ids.sort_by(|a, b| template_freqs[*b].cmp(&template_freqs[*a]));
                }
                sort_treshold *= 4;
            }

            for (t_i, res) in ac.find_iter(&line).flat_map(|res| {
                unique_patterns_ids[res.pattern().as_usize()]
                    .iter()
                    .zip(iter::repeat(res))
            }) {
                // The same template can be found multiple times in the same line
                if tried.contains(t_i) {
                    continue;
                }
                positions.push(res.start());
                positions.push(res.end());

                let mut current_pos = res.end();
                let mut found = true;
                for part in templates[*t_i].iter().skip(1) {
                    match find_subarray_from(&line, part, current_pos) {
                        Some(pos) => {
                            current_pos = pos;
                            positions.push(current_pos);
                            current_pos += part.len();
                            positions.push(current_pos);
                        }
                        None => {
                            positions.clear();
                            tried.push(*t_i);
                            found = false;
                            break;
                        }
                    }
                }
                if found {
                    template_id = t_i + 1;
                    template_freqs[*t_i] += 1;
                    break;
                }
            }
            positions.insert(0, 0);
            positions.push(line.len());

            if template_id == 0 {
                eprintln!("{:?}", str::from_utf8(&line).unwrap());
                eprintln!("{:?}", positions);
                panic!("Template not found");
            }

            let mut blk_id: Option<&[u8]> = None;
            let mut extra_blk_ids: Vec<&[u8]> = vec![];

            if template_id == 30 {
                let mut ids = line[positions[4]..positions[5]]
                    .trim_ascii()
                    .split(|c: &u8| *c == b' ');
                blk_id = Some(ids.next().unwrap());
                extra_blk_ids = ids.collect();
            } else {
                let blk_end_patterns: &[_] = b" .'";
                for (i, j) in (0..positions.len())
                    .step_by(2)
                    .zip((1..positions.len()).step_by(2))
                {
                    let param: &[u8] = &line[positions[i]..positions[j]];
                    if let Some(p) = find_subarray(param, b"blk_") {
                        blk_id = Some(
                            param[p..]
                                .split(|c: &u8| blk_end_patterns.contains(c))
                                .next()
                                .unwrap(),
                        );
                    }
                }
            }

            let timestamp = if &line[..2] == b"du" {
                // last few lines in log file do not have a timestamp
                prev_timestamp // assume that the lines without timestamp occur at the same time as the logs before
            } else {
                // timestamp format in logs: 081111 111607
                let year = 2000 + i32::from_radix_10(&line[0..2]).0;
                let month = u32::from_radix_10(&line[2..4]).0;
                let day = u32::from_radix_10(&line[4..6]).0;
                let hour = u32::from_radix_10(&line[7..9]).0;
                let minute = u32::from_radix_10(&line[9..11]).0;
                let second = u32::from_radix_10(&line[11..13]).0;

                let datetime = NaiveDate::from_ymd_opt(year, month, day)
                    .unwrap()
                    .and_hms_opt(hour, minute, second)
                    .unwrap()
                    .and_utc()
                    .timestamp();
                prev_timestamp = datetime;
                datetime
            };

            let blk_id = blk_id.expect("Block ID not found");
            let blk_id_num = blk_to_i(blk_id);
            let label = labels.get(&blk_id_num).expect("Label not found");

            write_output(&mut w, line_id, template_id, blk_id, timestamp, label)?;

            for extra_blk_id in extra_blk_ids.iter() {
                let blk_id_num = blk_to_i(extra_blk_id);
                let label = labels.get(&blk_id_num).expect("Label not found");

                write_output(&mut w, line_id, template_id, extra_blk_id, timestamp, label)?;
            }
        }

        w.flush()?;
    }
    Ok(())
}

// The output is wrapped in a Result to allow matching on errors.
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Split<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).split(b'\n'))
}
