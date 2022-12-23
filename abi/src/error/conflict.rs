// use std::{convert::Infallible, str::FromStr, collections::HashMap};

use chrono::{DateTime, Utc};
// use regex::Regex;

// #[derive(Debug, Clone)]
// pub enum ReservationConflictInfo {
//     Parsed(ReservationConflict),
//     Unparsed(String),
// }

#[derive(Debug, Clone)]
pub struct ReservationConflict {
    pub a: ReservationWindow,
    pub b: ReservationWindow,
}

#[derive(Debug, Clone)]
pub struct ReservationWindow {
    pub rid: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

// impl FromStr for ReservationConflictInfo {
//     type Err = Infallible;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         if let Ok(conflict) = s.parse() {
//             Ok(ReservationConflictInfo::Parsed(conflict))
//         } else {
//             Ok(ReservationConflictInfo::Unparsed(s.into()))
//         }
//     }
// }

// impl FromStr for ReservationConflict {
//     type Err = ();

//     /**
//      * "Key (resource_id, timespan)=(Ocean view room 518, [\"2022-12-26 22:00:00+00\",\"2022-12-31 07:00:00+00\")) conflicts with existing key
//      * (resource_id, timespan)=(Ocean view room 518, [\"2022-12-25 22:00:00+00\",\"2022-12-30 07:00:00+00\"))."
//      */
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         let re = Regex::new(r#"\((?P<k1>[a-zA-Z0-9_-]+)\s*,\s*(?P<k2>[a-zA-Z0-9_-]+)\)=\((?P<v1>[a-zA-Z0-9_-]+)\s*,\s*\[(?P<v2>[^\)\]]+)"#).unwrap();

//         let mut maps = vec![];

//         for cap in re.captures_iter(s) {
//             let mut map  = HashMap::new();
//             map.insert(cap["k1"].to_string(), cap["v1"].to_string());
//             map.insert(cap["k2"].to_string(), cap["v2"].to_string());
//             maps.push(map);
//         }

//         if maps.len() != 2 {
//             return Err(());
//         }

//         Ok(ParsedInfo {
//             a: maps[0].clone(),
//             b: maps[1].clone(),
//         })
//     }
// }

// pub struct ParsedInfo {
//     a: HashMap<String, String>,
//     b: HashMap<String, String>,
// }
