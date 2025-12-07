use SGP4_Rust::propagation::SatRec;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn load_tles_to_sat_rec(path: &str) -> Vec<SatRec> {
    let file = File::open(path).expect("Cannot open TLE file");
    let reader = BufReader::new(file);

    let mut sat_recs: Vec<SatRec> = Vec::new();
    let mut line_vec: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line.expect("Could not read line");
        line_vec.push(line);
        if line_vec.len() == 2 {
            let tle_line_1 = line_vec.remove(0);
            let tle_line_2 = line_vec.remove(0);
            sat_recs.push(SatRec::twoline2rv(&*tle_line_1, &*tle_line_2, "wgs84"))
        }
    }

    sat_recs
}
