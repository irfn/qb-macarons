extern crate rusqlite;

use rusqlite::{Connection, Result};
use rusqlite::NO_PARAMS;
use regex::{RegexSetBuilder, RegexSet};

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::env;
use clap::{Arg, App};

fn main() -> Result<()> {
    let matches = App::new("qb-macarons").version("1.0")
        .author("Irfan Shah<irfn@protonmail.com>")
        .about("Does awesome things")
        .arg(Arg::with_name("command")
             .help("Command that needs to be executed")
             .required(true)
             .index(1))
        .arg(Arg::with_name("db")
             .help("Sets the cookie database to use")
             .index(2))
        .arg(Arg::with_name("cfg_home")
             .help("Sets the config home")
             .index(3))
        
        .get_matches();
    let db = matches.value_of("db").unwrap_or("test.db");
    println!("db in use: {}", db);

    let command = matches.value_of("command").unwrap_or("macarons");
    let home = env::var("HOME").unwrap();
    //todo: default based on current os. perhaps $XDG_CONFIG_HOME
    let default_cfg_home = format!("{}/.qutebrowser/", home);
    let cfg_home = matches.value_of("cfg_home").unwrap_or(&default_cfg_home);
    println!("cfg home: {}", cfg_home);

    let mut conn = Connection::open(db)?;
    
    if command == "list-macarons" {
        for dom in filtered_hosts(&mut conn, cfg_home) {
            println!("Found domain {:?}", dom);
        }
    } else if command == "clear-cookies" {
        println!("clearing");
        let filtered = filtered_hosts(&mut conn, cfg_home);
        let result = clear_cookies(&mut conn, filtered);
        match result {
            Ok(_) => println!("Cookies were cleared"),
            Err(err) => println!("Clearing cookies failed: {}", err),
        }

    } else if command == "preview-clear-cookies" {
        let filtered = filtered_hosts(&mut conn, cfg_home);
        for dom in preview_clear_cookies(&mut conn, filtered) {
            println!("domain {:?}", dom);
        }
    }
    Ok(())
}

fn preview_clear_cookies(conn: &Connection, macarons: Vec<String>) -> Result<Vec<String>> {
    let badstmt = format!("select host_key from cookies where host_key not in ('{}')", macarons.join("','"));
    let mut stmt = conn.prepare(&badstmt)?;
    let mut hosts = Vec::new();
    let rows = stmt.query_map(NO_PARAMS, |row| row.get(0))?;
    for name_result in rows {
        hosts.push(name_result?);
    };
    return Ok(hosts)
}

fn clear_cookies(conn: &mut Connection, macarons: Vec<String>) -> Result<()> {
    let tx = conn.transaction()?;
    let deletestmt =  format!("delete from cookies where host_key not in ('{}')", macarons.join("','"));
    println!("{}",deletestmt);
    let result = tx.execute(&deletestmt, NO_PARAMS);
    match result {
        Ok(updated) => println!("{} rows were deleted", updated),
        Err(err) => println!("delete failed: {}", err),
    }
    return tx.commit()
 }

fn filtered_hosts(conn: &mut Connection, cfg_home: &str) -> Vec<String> {
    let set = whitelist(cfg_home);
    let filtered: Vec<_> = all_hosts(conn).unwrap().into_iter().filter(|host| set.is_match(host)).collect();
    return filtered
}

fn all_hosts(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("select distinct host_key from cookies order by host_key;")?;
    let mut hosts = Vec::new();
    let rows = stmt.query_map(NO_PARAMS, |row| row.get(0))?;
    for name_result in rows {
        hosts.push(name_result?);
    };
    return Ok(hosts)
}

fn whitelist(cfg_home: &str) -> RegexSet {
    if let Ok(lines) = read_lines(format!("{}/macarons", cfg_home)) {
        let mut regexes = Vec::new();
        for line in lines {
            if let Ok(regx) = line {
                regexes.push(regx);
            }
        }
        return RegexSetBuilder::new(regexes).build().unwrap()
    }
    return RegexSetBuilder::new(&[
        "^$",
    ]).build().unwrap()
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
