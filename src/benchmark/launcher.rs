use core::fmt;
use std::fs;

use crate::{
    benchmark::{
        epd_reader::{self, EpdRead},
        scoring,
    }, entity::engine::component::config::config, ui::notation::{epd, san}
};

use super::{epd_reader::EpdFileReaderError, scoring::EpdScore};

fn list_epd_files_in_folder(epd_folder: &str) -> Result<Vec<String>, std::io::Error> {
    let mut epd_files = vec![];

    // Read the directory
    for entry in fs::read_dir(epd_folder)? {
        let entry = entry?;
        let path = entry.path();
        // Check if the file has a ".epd" extension

        if let Some(extension) = path.extension() {
            if extension == "epd" {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        epd_files.push(file_name_str.to_string());
                    }
                }
            }
        }
    }

    Ok(epd_files)
}

#[derive(Debug)]
pub struct EpdResult<'a> {
    file_path: String,
    result: Vec<(&'a epd::Epd, EpdScore)>,
}
impl <'a> EpdResult<'a> {
    pub fn new(file_path: String, result: Vec<(&'a epd::Epd, EpdScore)>) -> Self {
        Self {
            file_path, 
            result,
        }
    }
    pub fn total(&self) -> f64 {
        let total: Vec<f64> = self.result.iter().map(|(_epd, epd_score)| epd_score.score()).collect();
        total.iter().sum()
    }
}
impl fmt::Display for EpdResult<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "file: {}", self.file_path)?;
        let mut epd_total = scoring::EpdScore::default();
        self.result.iter().for_each(|(epd, epd_score)| {
            writeln!(f, "{} {}", epd_score, epd).unwrap();
            epd_total.am_count += epd_score.am_count;
            epd_total.am_ok += epd_score.am_ok;
            epd_total.bm_count += epd_score.bm_count;
            epd_total.bm_ok += epd_score.bm_ok;            
        });
        writeln!(f, "{}", epd_total)?;
        writeln!(f, "total: {:.3}", self.total())
    }
}

#[derive(Debug)]
pub struct EpdData {
    file_path: String,
    epds: Vec<epd::Epd>,
}
impl std::fmt::Display for EpdData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let epds_as_str: Vec<String> = self.epds.iter().map(|epd| epd.to_string()).collect();
        let separator = "------------------------";
        write!(
            f,
            "{}{}{}\n{}\n{}",
            separator,
            self.file_path,
            separator,
            epds_as_str.join("\n"),
            separator
        )
    }
}
impl EpdData {
    pub fn new(file_path: String, epds: Vec<epd::Epd>) -> Self {
        Self { file_path, epds }
    }
    pub fn file_path(&self) -> String {
        self.file_path.clone()
    }
    pub fn epds(&self) -> &Vec<epd::Epd> {
        &self.epds
    }
}
pub fn benchmark(epd_folder: &str) -> Result<Vec<EpdData>, EpdFileReaderError> {
    let data_all_files_or_error = read_epds_from_folder(epd_folder);
    let conf_depth = 3;
    let max_time_sec = 3;
    let engine_conf = config::IDDFSConfig::new(conf_depth, config::IddfsFeatureConf::default(), config::AlphabetaFeatureConf::default());
    let constraint = scoring::Constraint::new(max_time_sec);
    let mut results: Vec<EpdResult> = vec![];
    if let Ok(data_all_files) = &data_all_files_or_error {
        for data_per_file in data_all_files {
            let epd_with_score = scoring::scoring(data_per_file, &engine_conf, &constraint);
            let epd_result = EpdResult::new(data_per_file.file_path(), epd_with_score);
            results.push(epd_result);
        }
        println!("----------------");
        println!("Benchmark results");
        println!("----------------");
        println!("am should be 0, bm should be 1");
        for result in results {
            println!("{}", result);
        }
        println!("----------------");
    }
    data_all_files_or_error
}
pub fn read_epds_from_folder(epd_folder: &str) -> Result<Vec<EpdData>, EpdFileReaderError> {
    let lang = &san::Lang::LangEn;
    let mut v: Vec<EpdData> = vec![];
    let files_str = list_epd_files_in_folder(epd_folder)?;

    for file_str in files_str.iter() {
        let file = format!("{}/{}", epd_folder, file_str);
        let epd_file = epd_reader::EpdFile(file.clone());
        let epds = epd_file.epd_read(lang)?;
        v.push(EpdData::new(file, epds));
    }
    Ok(v)
}
